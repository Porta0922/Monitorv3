use std::sync::Arc;
use chrono::Utc;
use crate::monitoring::MonitoringLoop;
use crate::is_running_in_session_0;
use super::{TaskContext, TaskInterval, live_sleep};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let is_session_0 = is_running_in_session_0();
    if is_session_0 {
        tracing::info!("Skipping open app snapshots task (requires user session)");
        return tokio::spawn(std::future::pending::<()>());
    }

    tokio::spawn(async move {
        let mut monitor = MonitoringLoop::new();
        let mut last_fingerprint = String::new();

        loop {
            live_sleep(&context, TaskInterval::RunningApps).await;

            let apps = monitor.capture_open_apps();
            let fingerprint = apps
                .iter()
                .map(|app| format!("{}|{}|{}", app.app_name, app.window_count, app.primary_title))
                .collect::<Vec<_>>()
                .join("||");

            if fingerprint == last_fingerprint {
                continue;
            }
            last_fingerprint = fingerprint;

            let running_apps_payload = context.build_event_envelope(
                "running_apps",
                1,
                serde_json::json!({
                    "detected_at": Utc::now().to_rfc3339(),
                    "apps": apps.into_iter().map(|app| serde_json::json!({
                        "app_name": app.app_name,
                        "primary_title": app.primary_title,
                        "window_count": app.window_count,
                        "exe_path": app.exe_path,
                        "exe_hash": app.exe_hash,
                    })).collect::<Vec<_>>()
                }),
            );

            context.publish_or_cache("running_apps", running_apps_payload).await;
        }
    })
}
