use std::sync::Arc;
use tokio::time::Instant;
use crate::monitoring::ResourceMonitor;
use crate::is_running_in_session_0;
use super::{TaskContext, TaskInterval, live_sleep};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut resource_monitor = ResourceMonitor::new();
        let mut last_summary_instant = Instant::now();
        let is_session_0 = is_running_in_session_0();

        loop {

            let key_stats = context.keystroke_tracker.get_stats().await;
            let resources = resource_monitor.capture_snapshot();
            let unavailable=["memory_state","disks_state","network_state"].iter().any(|key|resources.system_metrics[*key]=="unavailable");
            crate::capture_diagnostics::record("resource_logger",if unavailable {"capture_error"} else {"capture_ok"},if unavailable {Some("Uno o más campos no están disponibles; consultar estados por recurso")} else {None});
            crate::web::update_local_resources(resources.system_metrics.clone());
            
            let now_instant = Instant::now();
            let mut elapsed_secs = now_instant.duration_since(last_summary_instant).as_secs();
            last_summary_instant = now_instant;

            if elapsed_secs > 70 {
                elapsed_secs = 60;
            }

            let (status, idle_secs, active_secs) = if !crate::runtime::input_available() {
                (if is_session_0 { "service" } else { "online" }, 0, 0)
            } else {
                (
                    if key_stats.is_idle { "idle" } else { "active" },
                    if key_stats.is_idle { elapsed_secs as i64 } else { 0 },
                    if key_stats.is_idle { 0 } else { elapsed_secs as i64 }
                )
            };

            let summary_payload = context.build_event_envelope(
                "input_summary",
                1,
                serde_json::json!({
                    "keys_count": if !crate::runtime::input_available() { 0 } else { key_stats.keystroke_count },
                    "mouse_moves_count": if !crate::runtime::input_available() { 0 } else { key_stats.mouse_moves_count },
                    "clicks_count": if !crate::runtime::input_available() { 0 } else { key_stats.mouse_clicks_count },
                    "idle_seconds": idle_secs,
                    "active_seconds": active_secs,
                    "status": status,
                    "system_metrics": resources.system_metrics,
                    "cpu_percent": resources.cpu_percent,
                    "memory_used_mb": resources.memory_used_mb,
                    "memory_percent": resources.memory_percent,
                    "top_process_name": resources.top_process_name,
                    "top_process_cpu_percent": resources.top_process_cpu_percent,
                    "top_process_memory_mb": resources.top_process_memory_mb,
                    "is_service": is_session_0,
                    "input_available": crate::runtime::input_available(),
                }),
            );

            context.publish_or_cache("heartbeat", summary_payload).await;
            if !is_session_0 {
                context.keystroke_tracker.reset_minute_counters().await;
            }
            live_sleep(&context, TaskInterval::ResourceLogger).await;
        }
    })
}
