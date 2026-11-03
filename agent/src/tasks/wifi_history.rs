use std::sync::Arc;
use std::sync::atomic::Ordering;
use chrono::Utc;
use crate::wifi_detection::WifiMonitor;
use super::{TaskContext, TaskInterval, live_sleep};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut wifi_monitor = WifiMonitor::new();
        // Force an immediate state broadcast so any server that just started
        // learns the current WiFi state within the first scan.
        wifi_monitor.force_resend();

        loop {
            live_sleep(&context, TaskInterval::Wifi).await;

            // If RabbitMQ reconnected since last scan, force a state re-broadcast.
            if context.wifi_resend_flag.swap(false, Ordering::Relaxed) {
                wifi_monitor.force_resend();
            }

            match wifi_monitor.scan_and_detect_change().await {
                Ok(Some(snapshot)) => {
                    crate::capture_diagnostics::record("wifi_history", "capture_ok", None);
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
                Ok(None) => { crate::capture_diagnostics::record("wifi_history", "no_observation", None); }
                Err(e) => {
                    crate::capture_diagnostics::record("wifi_history", "capture_error", Some(&e.to_string()));
                    tracing::warn!("WiFi scan error: {}", e);
                }
            }
        }
    })
}
