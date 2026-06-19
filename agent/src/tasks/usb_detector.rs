use std::sync::Arc;
use tokio::time::Duration;
use crate::usb_detection::UsbMonitor;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let is_session_0 = is_running_in_session_0();
    if cfg!(windows) && !is_session_0 {
        tracing::info!("Skipping USB detection task (handled by service)");
        return tokio::spawn(std::future::pending::<()>());
    }

    tokio::spawn(async move {
        let mut usb_monitor = UsbMonitor::new();
        let mut interval = skip_interval(Duration::from_secs(60)); // Check every 60 seconds
        loop {
            interval.tick().await;
            
            match usb_monitor.scan_devices().await {
                Ok(events) => {
                    for mut event in events {
                        event.device_id = uuid::Uuid::parse_str(&context.device_id).unwrap_or_default();

                        let usb_payload = context.build_event_envelope(
                            "usb",
                            1,
                            serde_json::json!({
                                "device_name": event.usb_device.device_name,
                                "serial_number": event.usb_device.serial_number,
                                "action": match event.action {
                                    crate::usb_detection::UsbAction::Connected => "IN",
                                    crate::usb_detection::UsbAction::Disconnected => "OUT",
                                },
                                "timestamp": event.timestamp.to_rfc3339(),
                            }),
                        );

                        context.publish_or_cache("usb", usb_payload).await;
                    }
                }
                Err(e) => {
                    tracing::warn!("USB scan error: {}", e);
                }
            }
        }
    })
}
