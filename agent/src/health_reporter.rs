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
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let handles = supervisor.handles();
            let _config = config_manager.get().await;

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

            let total_unsynced = context.cache.get_unsynced_event_count().await.unwrap_or(0);
            let agent_uptime = AGENT_START_TIME.lock().unwrap().elapsed().as_secs();

            let degraded = handles.iter().any(|h| !h.running);
            let status = if degraded { "degraded" } else { "healthy" };

            let health_payload = context.build_event_envelope(
                "agent_health",
                1,
                serde_json::json!({
                    "status": status,
                    "config_version": config_manager.version(),
                    "agent_uptime_seconds": agent_uptime,
                    "tasks": tasks_status,
                    "cache_unsynced_events": total_unsynced,
                    "version": env!("CARGO_PKG_VERSION"),
                    "timestamp": Utc::now().to_rfc3339(),
                }),
            );

            context.publish_or_cache("health", health_payload).await;
            tracing::debug!("[HealthReporter] Published health report (status: {})", status);
        }
    });
}
