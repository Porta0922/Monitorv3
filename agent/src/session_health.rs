//! Ephemeral session diagnostics, separate from durable activity and its producer lease.
use std::{collections::BTreeMap, sync::{Arc, LazyLock, Mutex}, time::{Duration, Instant}};
use serde_json::{json, Value};
use crate::{tasks::TaskContext, task_supervisor::TaskSupervisor, session_relay::SessionRelay};
static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Observation { sequence: u64, received: Instant, report: Value }
static SESSIONS: LazyLock<Mutex<BTreeMap<String, Observation>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));

pub fn accept(instance: &str, report: &Value, principal: Option<&str>) -> std::io::Result<()> {
    let invalid=||std::io::Error::other("Invalid session diagnostic");
    uuid::Uuid::parse_str(instance).map_err(|_|invalid())?;
    if report.to_string().len()>16384 {return Err(invalid());}
    let sequence=report["sequence"].as_u64().filter(|n|*n>0).ok_or_else(invalid)?;
    let sent=report["sent_at"].as_str().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok()).ok_or_else(invalid)?;
    if (chrono::Utc::now().signed_duration_since(sent).num_seconds()).abs()>30 {return Err(invalid());}
    if !report["monitoring_paused"].is_boolean() || !report["input_available"].is_boolean()
        || !(report["pending_events"].is_null() || report["pending_events"].as_u64().is_some()) {return Err(invalid());}
    let fingerprint=report["policy_fingerprint"].as_str().ok_or_else(invalid)?;
    if fingerprint.len()!=64 || !fingerprint.bytes().all(|b|b.is_ascii_hexdigit()) {return Err(invalid());}
    let tasks=report["tasks"].as_array().ok_or_else(invalid)?;
    if tasks.len()>16 || tasks.iter().any(|t|!t["name"].as_str().is_some_and(|n|n.len()<=128) || !t["running"].is_boolean()) {return Err(invalid());}
    for task in tasks {
        if let Some(capture)=task.get("capture") {
            if !matches!(capture["state"].as_str(),Some("capture_ok"|"capture_error"|"no_observation"|"unknown"))
                || !(capture["last_error"].is_null() || capture["last_error"].as_str().is_some_and(|s|s.len()<=1024))
                || !(capture["observed_at"].is_null() || capture["observed_at"].as_str().is_some_and(|s|chrono::DateTime::parse_from_rfc3339(s).is_ok())) {return Err(invalid());}
        }
    }
    let scoped_instance=format!("{}:{instance}",principal.unwrap_or("legacy"));
    let mut sessions=SESSIONS.lock().unwrap_or_else(|e|e.into_inner());
    sessions.retain(|_,v|v.received.elapsed()<Duration::from_secs(900));
    if sessions.get(&scoped_instance).is_some_and(|old|sequence<=old.sequence) {return Err(invalid());}
    if !sessions.contains_key(&scoped_instance) && sessions.len()>=16 {return Err(std::io::Error::other("Session diagnostics capacity reached"));}
    // Copy only the diagnostic contract, never session-supplied arbitrary fields or secrets.
    let report=json!({"principal":principal,"instance_id":instance,"received_at":chrono::Utc::now().to_rfc3339(),
        "monitoring_paused":report["monitoring_paused"],"input_available":report["input_available"],
        "pending_events":report["pending_events"],"policy_fingerprint":fingerprint,
        "tasks":tasks.iter().map(|t|json!({"name":t["name"],"running":t["running"],"capture":t.get("capture").map(|c|json!({"state":c["state"],"last_error":c["last_error"],"observed_at":c["observed_at"]}))})).collect::<Vec<_>>()});
    sessions.insert(scoped_instance,Observation{sequence,received:Instant::now(),report});
    Ok(())
}
pub fn snapshot()->Value {
    let mut sessions=SESSIONS.lock().unwrap_or_else(|e|e.into_inner());
    sessions.retain(|_,v|v.received.elapsed()<Duration::from_secs(900));
    let mut instances=Vec::new();let mut bytes=0usize;let mut truncated=false;
    for observation in sessions.values() {
        let mut report=observation.report.clone();report["stale"]=json!(observation.received.elapsed()>Duration::from_secs(90));
        bytes+=report.to_string().len();
        if bytes>90000 {truncated=true;break;}
        instances.push(report);
    }
    json!({"truncated":truncated,"relay_enabled":crate::runtime::relay_enabled() || std::env::var_os("AGENT_SESSION_REGISTRY").is_some(),"producer_mode":if std::env::var_os("AGENT_SESSION_REGISTRY").is_some() {"per_principal"} else {"single_lease"},
        "instances":instances})
}
pub fn spawn(context:Arc<TaskContext>,supervisor:Arc<TaskSupervisor>,relay:Arc<SessionRelay>)->tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut timer=crate::tasks::skip_interval(Duration::from_secs(30));
        loop {
            timer.tick().await;let sequence=SEQUENCE.fetch_add(1,std::sync::atomic::Ordering::Relaxed)+1;
            let config=context.config_manager.get().await;
            let policy=serde_json::to_value(&config).expect("Policy serialization");
            let fingerprint={use sha2::Digest;format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&policy).expect("Policy serialization")))};
            let tasks=supervisor.handles().into_iter().map(|h|{
                let mut capture=crate::capture_diagnostics::latest(&h.name);
                if let Some(error)=capture["last_error"].as_str() {capture["last_error"]=json!(error.chars().take(256).collect::<String>());}
                json!({"name":h.name,"running":h.running,"capture":capture})
            }).collect::<Vec<_>>();
            let report=json!({"sequence":sequence,"sent_at":chrono::Utc::now().to_rfc3339(),
                "monitoring_paused":config.monitoring_paused,"input_available":crate::runtime::input_available(),
                "pending_events":context.cache.get_unsynced_event_count().await.ok(),"policy_fingerprint":fingerprint,"tasks":tasks});
            if let Err(error)=relay.report_health(&context.device_id,report).await {tracing::warn!(%error,"Session diagnostic unavailable");}
        }
    })
}
