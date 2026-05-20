use std::sync::Arc;
use tokio::time::Duration;
use chrono::Utc;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) {
    tokio::spawn(async move {
        let mut hb = skip_interval(Duration::from_secs(60));
        let mut last_idle_state: Option<bool> = None;

        loop {
            hb.tick().await;
            
            // Re-check session 0 every time to be safe
            let is_session_0 = is_running_in_session_0();

            let (status, idle_seconds, total_idle_today) = if is_session_0 {
                ("service", 0, 0)
            } else {
                context.keystroke_tracker.update_idle_status().await;
                let stats = context.keystroke_tracker.get_stats().await;
                (if stats.is_idle { "idle" } else { "active" }, stats.idle_duration_seconds, stats.total_inactive_seconds_today)
            };

            let is_idle = status == "idle";

            let heartbeat = context.build_event_envelope(
                "heartbeat",
                1,
                serde_json::json!({
                    "last_seen": Utc::now().to_rfc3339(),
                    "status": status,
                    "idle_seconds": idle_seconds,
                    "total_idle_seconds_today": total_idle_today,
                    "is_service": is_session_0,
                }),
            );

            context.publish_or_cache("heartbeat", heartbeat).await;

            // Explicit idle/active transitions (Skip if service)
            if !is_session_0 && last_idle_state != Some(is_idle) {
                let transition = context.build_event_envelope(
                    "idle_state_change",
                    1,
                    serde_json::json!({
                        "status": status,
                        "changed_at": Utc::now().to_rfc3339(),
                    }),
                );
                context.publish_or_cache("heartbeat", transition).await;
                last_idle_state = Some(is_idle);
            }
        }
    });
}
