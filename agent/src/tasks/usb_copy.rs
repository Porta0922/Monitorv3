use std::sync::Arc;
use tokio::time::Duration;
use crate::usb_file_copy_detection::UsbFileCopyMonitor;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let is_session_0 = is_running_in_session_0();
    if cfg!(windows) && is_session_0 {
        tracing::info!("Skipping USB copy detection task (handled by user agent)");
        return tokio::spawn(std::future::pending::<()>());
    }

    tokio::spawn(async move {
        let mut detector = UsbFileCopyMonitor::new(900);
        let mut interval = skip_interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            // Wider lookback/cap helps on larger USB volumes where recursive scans
            // can be slower and recent writes might otherwise fall out of the window.
            let findings = match detector.scan_recent_writes(1800, 200).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::warn!("USB copy detector scan failed: {}", e);
                    continue;
                }
            };

            if !findings.is_empty() {
                tracing::info!(
                    "USB copy detector found {} candidate file write(s)",
                    findings.len()
                );
            }

            for finding in findings {
                let drive_letter = finding.drive_letter;
                let file_name = finding.file_name;
                let file_path = finding.file_path;
                let size_bytes = finding.size_bytes;
                let modified_utc = finding.modified_utc;
                let fingerprint = finding.fingerprint;
                let description = format!(
                    "Copia a USB detectada: {} en {} ({} bytes)",
                    file_name,
                    drive_letter,
                    size_bytes,
                );

                let security_payload = context.build_event_envelope(
                    "security",
                    1,
                    serde_json::json!({
                        "alert_type": "USB_FILE_COPY_DETECTED",
                        "description": description,
                        "app_name": "usb_copy_monitor",
                        "query_name": "usb_file_copy_detected",
                        "query_pack": "usb_data_loss_prevention",
                        "mitre_technique": "T1052.001",
                        "severity": "HIGH",
                        "raw_data": {
                            "source": "usb_copy_monitor",
                            "drive_letter": drive_letter,
                            "file_name": file_name,
                            "file_path": file_path,
                            "size_bytes": size_bytes,
                            "modified_utc": modified_utc.to_rfc3339(),
                        },
                        "event_fingerprint": fingerprint,
                    }),
                );

                context.publish_or_cache("security", security_payload).await;
            }
        }
    })
}
