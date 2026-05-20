use std::sync::Arc;
use tokio::time::Duration;
use chrono::Utc;
use crate::monitoring::MonitoringLoop;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) {
    let is_session_0 = is_running_in_session_0();
    // Open app snapshots require a user session
    if is_session_0 {
        tracing::info!("Skipping open app snapshots task (requires user session)");
        return;
    }

    tokio::spawn(async move {
        let mut monitor = MonitoringLoop::new();
        let mut interval = skip_interval(Duration::from_secs(60)); // Slower for "silence"
        let mut last_fingerprint = String::new();

        loop {
            interval.tick().await;

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
    });
}
