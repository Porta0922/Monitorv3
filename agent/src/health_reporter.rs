use std::sync::Arc;
use std::sync::Mutex;
use std::sync::LazyLock;
use tokio::time::Duration;
use chrono::Utc;
use crate::config_manager::ConfigManager;
use crate::task_supervisor::TaskSupervisor;
use crate::tasks::TaskContext;

static AGENT_START_TIME: LazyLock<Mutex<std::time::Instant>> =
    LazyLock::new(|| Mutex::new(std::time::Instant::now()));

pub fn spawn_health_reporter(
    context: Arc<TaskContext>,
    supervisor: Arc<TaskSupervisor>,
    config_manager: Arc<ConfigManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let handles = supervisor.handles();
            let config = config_manager.get().await;

            let tasks_status: Vec<serde_json::Value> = handles
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "name": h.name,
                        "running": h.running,
                        "restart_count": h.restart_count,
                        "last_error": h.last_error,
                        "uptime_seconds": h.uptime_seconds,
                    })
                })
                .collect();

            let collectors: Vec<_> = ["window_activity", "heartbeat", "usb_detector", "usb_copy", "wifi_history",
                "running_apps", "inventory", "heatmap", "resource_logger", "security_osquery"].iter().map(|name| {
                let task = handles.iter().find(|h| h.name == *name);
                let state = if !crate::runtime::permits_task(name) { "unsupported_context" }
                    else if !config.is_monitor_enabled(name) { "disabled" }
                    else if task.is_some_and(|h|h.running) { "supervised" }
                    else if task.is_some() { "stopped" } else { "not_started" };
                serde_json::json!({"name":name,"state":state,"last_error":task.and_then(|h|h.last_error.as_deref()),"capture":crate::capture_diagnostics::latest(name)})
            }).collect();
            let total_unsynced = match context.cache.get_unsynced_event_count().await {
                Ok(count) => Some(count),
                Err(error) => { tracing::warn!(%error,"Cannot read pending queue size"); None }
            };
            let agent_uptime = AGENT_START_TIME.lock().unwrap().elapsed().as_secs();

            let degraded = total_unsynced.is_none() || handles.iter().any(|h| !h.running) || collectors.iter().any(|c|c["state"]=="supervised" && c["capture"]["state"]=="capture_error");
            let status = if degraded { "degraded" } else { "healthy" };

            let policy_value=serde_json::to_value(&config).expect("Agent policy serialization");
            let policy_fingerprint={use sha2::Digest;format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&policy_value).expect("Policy serialization")))};
            let health_payload = context.build_event_envelope(
                "agent_health",
                1,
                serde_json::json!({
                    "status": status,
                    "config_version": config_manager.version(),
                    "policy_fingerprint": policy_fingerprint,
                    "monitoring_paused": config.monitoring_paused,
                    "enabled_monitors": config.enabled_monitors,
                    "disabled_monitors": config.disabled_monitors,
                    "agent_uptime_seconds": agent_uptime,
                    "tasks": tasks_status,
                    "sessions": crate::session_health::snapshot(),
                    "collectors": collectors,
                    "capabilities": crate::runtime::capabilities(),
                    "cache_unsynced_events": total_unsynced,
                    "version": env!("CARGO_PKG_VERSION"),
                    "timestamp": Utc::now().to_rfc3339(),
                }),
            );

            context.publish_or_cache("health", health_payload).await;
            tracing::debug!("[HealthReporter] Published health report (status: {})", status);
        }
    })
}
