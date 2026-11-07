// Runs the actual agent transport/cache without installing monitors or services.
#[allow(dead_code)]
#[path = "../../../agent/src/rabbitmq_publisher.rs"]
mod rabbitmq_publisher;
#[allow(dead_code)]
#[path = "../../../agent/src/offline_cache.rs"]
mod offline_cache;
#[allow(dead_code)]
#[path = "../../../agent/src/outbox_drain.rs"]
mod outbox_drain;
#[allow(dead_code)]
#[path = "../../../agent/src/usb_detection.rs"]
mod usb_detection;
#[allow(dead_code)]
#[path = "../../../agent/src/network_identity.rs"]
mod network_identity;
#[allow(dead_code)]
#[path = "../../../agent/src/persistent_file.rs"]
mod persistent_file;
mod runtime {
 pub fn data_dir()->std::path::PathBuf{std::env::var("AGENT_DATA_DIR").expect("probe data dir").into()}
 pub fn is_session()->bool{false}
}
mod tasks {pub struct TaskContext {pub publisher:std::sync::Arc<tokio::sync::RwLock<Option<std::sync::Arc<crate::rabbitmq_publisher::RabbitMQPublisher>>>>}}

use rabbitmq_publisher::RabbitMQPublisher;
use offline_cache::OfflineCache;
use std::time::{Duration, Instant};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut url = std::env::var("RABBITMQ_URL").unwrap_or_default();
    let device = std::env::var("PROBE_DEVICE_ID").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    if std::env::var("PROBE_MANAGED_IDENTITY").as_deref()==Ok("true") {
        network_identity::initialize(uuid::Uuid::parse_str(&device)?).await?;
        url=network_identity::rabbit_url("");
        if network_identity::token("").is_empty(){return Err("Enrollment still pending".into());}
        if std::env::var("PROBE_ENROLL_ONLY").as_deref()==Ok("true"){println!("Native identity initialization completed");return Ok(());}
    }
    if let Ok(operation)=std::env::var("PROBE_FORBIDDEN_OPERATION") {
        let connection=lapin::Connection::connect(&url,lapin::ConnectionProperties::default()).await?;
        let channel=connection.create_channel().await?;
        let denied=match operation.as_str(){
            "consume"=>channel.basic_get("heartbeat_queue".into(),Default::default()).await.is_err(),
            "configure"=>channel.queue_declare("unauthorized_probe".into(),Default::default(),Default::default()).await.is_err(),
            _=>return Err("Unknown permission probe".into())
        };
        if !denied{return Err("Forbidden broker operation succeeded".into());}println!("Broker denied forbidden operation");return Ok(());
    }
    if let Ok(foreign)=std::env::var("PROBE_SPOOF_PAYLOAD_UUID") {
        let connection=lapin::Connection::connect(&url,lapin::ConnectionProperties::default()).await?;
        let channel=connection.create_channel().await?;channel.confirm_select(Default::default()).await?;
        let user=reqwest::Url::parse(&url)?.username().to_string();
        let data=serde_json::to_vec(&json!({"device_id":foreign,"event_id":uuid::Uuid::new_v4(),"boot_id":uuid::Uuid::new_v4(),"sequence":1,"schema_version":1,"event_type":"input_summary","timestamp":chrono::Utc::now().to_rfc3339(),"payload":{"status":"active","active_seconds":2,"idle_seconds":0}}))?;
        let confirm=channel.basic_publish("monitoring".into(),format!("monitoring.{device}.heartbeat").into(),lapin::options::BasicPublishOptions{mandatory:true,..Default::default()},&data,lapin::BasicProperties::default().with_user_id(user.into()).with_delivery_mode(2)).await?.await?;
        if !matches!(confirm,lapin::Confirmation::Ack(None)){return Err("Spoof probe was not routed".into());}
        println!("Forged payload delivered for consumer authorization check");return Ok(());
    }
    if std::env::var("PROBE_HOLD_FOR_REVOCATION").as_deref()==Ok("true") {
        let publisher=RabbitMQPublisher::connect(&url).await?;
        println!("Connected; waiting for revocation");
        tokio::time::sleep(Duration::from_secs(10)).await;
        if publisher.publish_event("heartbeat",json!({"device_id":device})).await.is_ok(){return Err("Revoked live connection still published".into());}
        println!("Revoked live connection rejected");return Ok(());
    }
    let expect_rejection=std::env::var("PROBE_EXPECT_REJECTION").as_deref()==Ok("true");
    if expect_rejection {
        let result=RabbitMQPublisher::connect(&url).await;
        if let Ok(p)=result {let event=json!({"device_id":device,"event_id":uuid::Uuid::new_v4().to_string(),"boot_id":uuid::Uuid::new_v4().to_string(),"sequence":1,"timestamp":chrono::Utc::now().to_rfc3339()});if p.publish_event("heartbeat",event).await.is_ok(){return Err("Unauthorized publication unexpectedly succeeded".into());}}
        println!("Expected rejection observed");return Ok(());
    }
    if let Ok(path)=std::env::var("PROBE_EVENTS_FILE") {
        let packets:Vec<serde_json::Value>=serde_json::from_slice(&std::fs::read(path)?)?;
        let publisher=RabbitMQPublisher::connect(&url).await?;
        for packet in &packets {
            let route=packet["route"].as_str().ok_or("Missing fixture route")?;
            if !matches!(route,"inventory"|"usb"|"wifi"){return Err("Unsupported fixture route".into());}
            publisher.publish_event(route,packet["event"].clone()).await?;
        }
        println!("Published {} synthetic fixture packets",packets.len());return Ok(());
    }
    let seconds: u64 = std::env::var("PROBE_SECONDS").unwrap_or("1".into()).parse()?;
    let path = std::env::var("PROBE_CACHE").unwrap_or("/tmp/agent-probe.sqlite".into());
    let cache = OfflineCache::new(&path, &[17u8; 32])?;
    let mut publisher: Option<RabbitMQPublisher> = None;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut produced = 0u64;
    let mut cached = 0u64;
    let backlog: u64 = std::env::var("PROBE_BACKLOG").unwrap_or("0".into()).parse()?;
    if backlog > 5000 { return Err("Probe backlog limit is 5000".into()); }
    for sequence in 0..backlog {
        cache.save_event("heartbeat", &json!({
            "event_id":uuid::Uuid::new_v4(),"event_type":"input_summary","schema_version":1,
            "boot_id":uuid::Uuid::new_v4(),"sequence":sequence+1,"device_id":device,
            "hostname":"agent-transport-probe","timestamp":chrono::Utc::now().to_rfc3339(),
            "payload":{"status":"active","active_seconds":2,"idle_seconds":0,
                "cpu_percent":42.5,"memory_used_mb":1024,"memory_percent":25}
        })).await?;
        produced += 1;
    }
    cached = cached.max(cache.get_unsynced_event_count().await?);
    println!("{}", json!({"probe_ready":true,"device_id":device}));
    loop {
        let active = Instant::now() < deadline;
        if active {
            let event = json!({"event_id":uuid::Uuid::new_v4().to_string(),
                "event_type":"input_summary", "schema_version":1,"boot_id":uuid::Uuid::new_v4().to_string(),"sequence":produced+1,
                "device_id":device,"hostname":"agent-transport-probe",
                "timestamp":chrono::Utc::now().to_rfc3339(),
                "payload":{"status":"active","active_seconds":2,"idle_seconds":0,
                           "cpu_percent":42.5,"memory_used_mb":1024,"memory_percent":25}});
            // Persist first so interruption of this probe also exercises recovery.
            cache.save_event("heartbeat", &event).await?;
            produced += 1;
        }
        if publisher.is_none() {
            publisher = RabbitMQPublisher::connect(&url).await.ok();
        }
        let mut delay = Duration::from_secs(2);
        if let Some(publisher) = publisher.as_ref() {
            let status = outbox_drain::drain_batch(&cache, |kind, event| async move {
                publisher.publish_event(&kind, event).await.map_err(|error| error.to_string())
            }).await?;
            if matches!(status, outbox_drain::DrainStatus::More) { delay = Duration::from_millis(250); }
        }
        let pending = cache.get_unsynced_event_count().await?;
        cached = cached.max(pending);
        println!("{}", json!({"produced":produced,"pending":pending}));
        if !active && pending == 0 { break; }
        if Instant::now() > deadline + Duration::from_secs(120) {
            return Err("broker recovery deadline exceeded; cache retained".into());
        }
        tokio::time::sleep(delay).await;
    }
    println!("{}", json!({"probe_passed":true,"device_id":device,"produced":produced,"max_pending":cached}));
    Ok(())
}
