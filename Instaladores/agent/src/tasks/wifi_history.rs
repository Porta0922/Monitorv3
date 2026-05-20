use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::time::Duration;
use chrono::Utc;
use crate::wifi_detection::WifiMonitor;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) {
    // Only run WiFi monitoring in Session 0 (Service) or if not on Windows
    let is_session_0 = is_running_in_session_0();
    if cfg!(windows) && !is_session_0 {
        tracing::info!("Skipping WiFi monitoring task (handled by service)");
        return;
    }

    tokio::spawn(async move {
        let mut wifi_monitor = WifiMonitor::new();
        // Force an immediate state broadcast so any server that just started
        // learns the current WiFi state within the first scan.
        wifi_monitor.force_resend();

        let mut interval = skip_interval(Duration::from_secs(120)); // Check every 120 seconds
        loop {
            interval.tick().await;

            // If RabbitMQ reconnected since last scan, force a state re-broadcast.
            if context.wifi_resend_flag.swap(false, Ordering::Relaxed) {
                wifi_monitor.force_resend();
            }

            match wifi_monitor.scan_and_detect_change().await {
                Ok(Some(snapshot)) => {
                    let wifi_payload = context.build_event_envelope(
                        "wifi",
                        1,
                        serde_json::json!({
                            "interface_name": snapshot.interface_name,
                            "state": snapshot.state,
                            "ssid": snapshot.ssid,
                            "bssid": snapshot.bssid,
                            "signal_percent": snapshot.signal_percent,
                            "timestamp": Utc::now().to_rfc3339(),
                        }),
                    );

                    context.publish_or_cache("wifi", wifi_payload).await;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("WiFi scan error: {}", e);
                }
            }
        }
    });
}
