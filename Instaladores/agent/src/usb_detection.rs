// USB/External Storage Device Detection
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub device_id: String,      // Hardware identifier
    pub vendor_id: String,       // VID
    pub product_id: String,      // PID
    pub serial_number: String,   // Serial
    pub device_name: String,     // User-friendly name
    pub volume_label: Option<String>, // Drive label
    pub capacity_bytes: Option<u64>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbEvent {
    pub device_id: Uuid,         // Agent device_id
    pub usb_device: UsbDevice,
    pub action: UsbAction,       // Connected or Disconnected
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UsbAction {
    #[serde(rename = "IN")]
    Connected,
    #[serde(rename = "OUT")]
    Disconnected,
}

pub struct UsbMonitor {
    previous_devices: Vec<UsbDevice>,
}

impl UsbMonitor {
    pub fn new() -> Self {
        Self {
            previous_devices: Vec::new(),
        }
    }

    /// Scan for connected USB/external storage devices
    pub async fn scan_devices(&mut self) -> Result<Vec<UsbEvent>, String> {
        let current_devices = self.get_connected_devices().await?;
        let mut events = Vec::new();

        // Detect new devices (connected)
        for device in &current_devices {
            if !self.previous_devices.iter().any(|d| d.device_id == device.device_id) {
                events.push(UsbEvent {
                    device_id: Uuid::new_v4(), // Will be set by caller
                    usb_device: device.clone(),
                    action: UsbAction::Connected,
                    timestamp: Utc::now(),
                });
            }
        }

        // Detect removed devices (disconnected)
        for device in &self.previous_devices {
            if !current_devices.iter().any(|d| d.device_id == device.device_id) {
                events.push(UsbEvent {
                    device_id: Uuid::new_v4(), // Will be set by caller
                    usb_device: device.clone(),
                    action: UsbAction::Disconnected,
                    timestamp: Utc::now(),
                });
            }
        }

        self.previous_devices = current_devices;
        Ok(events)
    }

    /// Get connected USB devices (platform-specific)
    async fn get_connected_devices(&self) -> Result<Vec<UsbDevice>, String> {
        #[cfg(target_os = "windows")]
        {
            Self::scan_windows_devices().await
        }

        #[cfg(target_os = "linux")]
        {
            Self::scan_linux_devices().await
        }

        #[cfg(target_os = "macos")]
        {
            Self::scan_macos_devices().await
        }
    }
}

#[cfg(target_os = "windows")]
impl UsbMonitor {
    async fn scan_windows_devices() -> Result<Vec<UsbDevice>, String> {
        use std::process::Command;
        use serde_json::Value;

        let mut devices = Vec::new();

        // Capture only physical USB storage devices (exclude internal NVMe/SATA).
        let mut cmd = Command::new("powershell");
        cmd.args(&[
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_DiskDrive | Where-Object { $_.PNPDeviceID -match '^USBSTOR\\\\' -or $_.InterfaceType -eq 'USB' } | Select-Object Model,PNPDeviceID,SerialNumber,DeviceID | ConvertTo-Json -Compress",
        ]);
        
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        
        let output = cmd.output().map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Ok(devices);
        }

        let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if output_str.is_empty() {
            return Ok(devices);
        }

        let json_value: Value = serde_json::from_str(&output_str).map_err(|e| e.to_string())?;

        let rows: Vec<&Value> = if let Some(arr) = json_value.as_array() {
            arr.iter().collect()
        } else {
            vec![&json_value]
        };

        for row in rows {
            let model = row
                .get("Model")
                .and_then(|v| v.as_str())
                .unwrap_or("USB Storage Device")
                .trim()
                .to_string();

            let pnp_device_id = row
                .get("PNPDeviceID")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            if pnp_device_id.is_empty() {
                continue;
            }

            let serial_raw = row
                .get("SerialNumber")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            let serial_from_pnp = pnp_device_id
                .split('\\')
                .last()
                .map(|s| s.trim().trim_matches('&').to_string())
                .unwrap_or_default();

            let serial_number = if !serial_raw.is_empty() {
                serial_raw
            } else if !serial_from_pnp.is_empty() {
                serial_from_pnp
            } else {
                "UNKNOWN".to_string()
            };

            let upper_pnp = pnp_device_id.to_uppercase();
            let vendor_id = extract_token_value(&upper_pnp, "VID_").unwrap_or_else(|| "UNKNOWN".to_string());
            let product_id = extract_token_value(&upper_pnp, "PID_").unwrap_or_else(|| "UNKNOWN".to_string());

            let device = UsbDevice {
                // Stable key for IN/OUT matching between scans.
                device_id: pnp_device_id.clone(),
                vendor_id,
                product_id,
                serial_number,
                device_name: model,
                volume_label: None,
                capacity_bytes: None,
                detected_at: Utc::now(),
            };
            devices.push(device);
        }

        Ok(devices)
    }
}

#[cfg(target_os = "windows")]
fn extract_token_value(source: &str, token: &str) -> Option<String> {
    let idx = source.find(token)?;
    let after = &source[idx + token.len()..];
    let value: String = after
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(target_os = "linux")]
impl UsbMonitor {
    async fn scan_linux_devices() -> Result<Vec<UsbDevice>, String> {
        use std::fs;
        use std::path::Path;

        let mut devices = Vec::new();
        let usb_path = Path::new("/sys/bus/usb/devices");

        if usb_path.exists() {
            for entry in fs::read_dir(usb_path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();

                if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy().to_string();

                    // Skip non-device entries
                    if !name.contains(":") {
                        continue;
                    }

                    let manufacturer = fs::read_to_string(path.join("manufacturer"))
                        .unwrap_or_else(|_| "Unknown".to_string())
                        .trim()
                        .to_string();

                    let product = fs::read_to_string(path.join("product"))
                        .unwrap_or_else(|_| "Unknown".to_string())
                        .trim()
                        .to_string();

                    let serial = fs::read_to_string(path.join("serial"))
                        .unwrap_or_else(|_| "Unknown".to_string())
                        .trim()
                        .to_string();

                    let device = UsbDevice {
                        device_id: name,
                        vendor_id: "UNKNOWN".to_string(),
                        product_id: "UNKNOWN".to_string(),
                        serial_number: serial,
                        device_name: format!("{} {}", manufacturer, product),
                        volume_label: None,
                        capacity_bytes: None,
                        detected_at: Utc::now(),
                    };

                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }
}

#[cfg(target_os = "macos")]
impl UsbMonitor {
    async fn scan_macos_devices() -> Result<Vec<UsbDevice>, String> {
        use std::process::Command;

        let mut devices = Vec::new();

        // Use system_profiler to get USB device info
        let output = Command::new("system_profiler")
            .args(&["SPUSBDataType", "-json"])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                // Parse JSON and extract USB devices
                if let Some(usb_data) = json.get("SPUSBDataType").and_then(|d| d.get(0)) {
                    if let Some(items) = usb_data.get("_items").and_then(|d| d.as_array()) {
                        for item in items {
                            if let (Some(name), Some(serial)) = (
                                item.get("_name").and_then(|n| n.as_str()),
                                item.get("serial_num").and_then(|s| s.as_str()),
                            ) {
                                let device = UsbDevice {
                                    device_id: format!("{}-{}", name, serial),
                                    vendor_id: "UNKNOWN".to_string(),
                                    product_id: "UNKNOWN".to_string(),
                                    serial_number: serial.to_string(),
                                    device_name: name.to_string(),
                                    volume_label: None,
                                    capacity_bytes: None,
                                    detected_at: Utc::now(),
                                };
                                devices.push(device);
                            }
                        }
                    }
                }
            }
        }

        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_usb_monitor_creation() {
        let monitor = UsbMonitor::new();
        assert!(monitor.previous_devices.is_empty());
    }

    #[tokio::test]
    async fn test_scan_devices() {
        let mut monitor = UsbMonitor::new();
        // This will work on systems with USB devices
        let result = monitor.scan_devices().await;
        assert!(result.is_ok());
    }
}