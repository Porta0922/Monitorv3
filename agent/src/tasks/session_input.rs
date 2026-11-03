//! Session-only counters. Resource metrics are emitted exclusively by the service.
use std::sync::Arc;
use tokio::time::Instant;
use super::{TaskContext, TaskInterval, live_sleep};
pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last = Instant::now();
        loop {
            live_sleep(&context, TaskInterval::ResourceLogger).await;
            let now = Instant::now();
            let seconds = now.duration_since(last).as_secs().min(60) as i64;
            last = now;
            let available=crate::runtime::input_available();
            if !available { crate::capture_diagnostics::record("session_input","capture_error",Some("Entrada nativa no disponible")); }
            if available {crate::capture_diagnostics::record("session_input","capture_ok",None);}
            context.keystroke_tracker.update_idle_status().await;
            let stats = context.keystroke_tracker.get_stats().await;
            let event = context.build_event_envelope("input_summary", 1, serde_json::json!({
                "keys_count": if available {stats.keystroke_count} else {0}, "mouse_moves_count": if available {stats.mouse_moves_count} else {0},
                "clicks_count": if available {stats.mouse_clicks_count} else {0},
                "idle_seconds": if available && stats.is_idle { seconds } else { 0 },
                "active_seconds": if available && !stats.is_idle { seconds } else { 0 },
                "status": if stats.is_idle { "idle" } else { "active" },
                "input_available": available, "is_service": false
            }));
            if context.publish_or_cache("heartbeat", event).await {
                context.keystroke_tracker.reset_minute_counters().await;
            }
        }
    })
}
