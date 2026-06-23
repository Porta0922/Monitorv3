use std::process::Command;

#[derive(Debug, Clone)]
pub struct WifiSnapshot {
    pub interface_name: String,
    pub state: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub signal_percent: Option<i32>,
}

pub struct WifiMonitor {
    last_key: Option<String>,
    last_signal: Option<i32>,
    /// Counts scans since last broadcast; forces a re-send every N scans even
    /// without a state change so a freshly-started server always learns the
    /// current WiFi state within a bounded interval.
    scans_since_broadcast: u32,
    periodic_broadcast_interval: u32,
}

impl WifiMonitor {
    pub fn new() -> Self {
        Self {
            last_key: None,
            last_signal: None,
            scans_since_broadcast: 0,
            // 10 scans × 60 s/scan = 10 minutes max before a periodic re-send.
            periodic_broadcast_interval: 10,
        }
    }

    /// Force the next call to `scan_and_detect_change` to report the current
    /// WiFi state regardless of whether it changed.  Call this after a
    /// RabbitMQ reconnect so the new server receives the current state quickly.
    pub fn force_resend(&mut self) {
        self.scans_since_broadcast = self.periodic_broadcast_interval;
    }

    pub async fn scan_and_detect_change(
        &mut self,
    ) -> Result<Option<WifiSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
        let Some(snapshot) = current_wifi_snapshot()? else {
            return Ok(None);
        };

        let current_key = format!(
            "{}|{}|{}|{}",
            snapshot.interface_name,
            snapshot.state,
            snapshot.ssid.as_deref().unwrap_or(""),
            snapshot.bssid.as_deref().unwrap_or(""),
        );

        let state_changed = self.last_key.as_ref() != Some(&current_key);

        if state_changed {
            self.last_key = Some(current_key);
            self.last_signal = snapshot.signal_percent;
            self.scans_since_broadcast = 0;
            return Ok(Some(snapshot));
        }

        Ok(None)
    }
}

impl Default for WifiMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
fn current_wifi_snapshot() -> Result<Option<WifiSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    let mut cmd = Command::new("netsh");
    cmd.args(["wlan", "show", "interfaces"]);
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    
    let output = cmd.output()?;

    if !output.status.success() {
        tracing::warn!("netsh wlan show interfaces failed (exit code: {}). Is a WiFi adapter available?", output.status.code().unwrap_or(-1));
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut interface_name: Option<String> = None;
    let mut state: Option<String> = None;
    let mut ssid: Option<String> = None;
    let mut bssid: Option<String> = None;
    let mut signal_percent: Option<i32> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || !line.contains(':') {
            continue;
        }

        let mut parts = line.splitn(2, ':');
        let key = parts.next().unwrap_or("").trim().to_lowercase();
        let value = parts.next().unwrap_or("").trim();

        if value.is_empty() {
            continue;
        }

        if (key == "name" || key == "nombre") && interface_name.is_none() {
            interface_name = Some(value.to_string());
            continue;
        }

        if (key == "state" || key == "estado") && state.is_none() {
            state = Some(value.to_lowercase());
            continue;
        }

        if key == "ssid" && ssid.is_none() {
            ssid = Some(value.to_string());
            continue;
        }

        if key == "bssid" && bssid.is_none() {
            bssid = Some(value.to_string());
            continue;
        }

        let looks_like_signal_key = key == "signal" || key == "senal" || key.contains("signal") || key.contains("se") && key.contains("al");
        if looks_like_signal_key && signal_percent.is_none() {
            signal_percent = parse_percent(value);
        }
    }

    if interface_name.is_none() {
        return Ok(None);
    }

    let state_value = state.unwrap_or_else(|| "unknown".to_string());
    let connected = state_value.contains("connected") || state_value.contains("conectado");

    Ok(Some(WifiSnapshot {
        interface_name: interface_name.unwrap_or_else(|| "Wi-Fi".to_string()),
        state: if connected { "connected".to_string() } else { "disconnected".to_string() },
        ssid: if connected { ssid } else { None },
        bssid: if connected { bssid } else { None },
        signal_percent: if connected { signal_percent } else { None },
    }))
}

#[cfg(target_os = "linux")]
fn current_wifi_snapshot() -> Result<Option<WifiSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    let output = match Command::new("/usr/bin/nmcli")
        .args(["-t", "-f", "active,ssid,bssid,signal,device", "dev", "wifi"])
        .output() {
            Ok(out) => out,
            Err(e) => {
                tracing::warn!("Failed to execute nmcli: {}", e);
                return Ok(None);
            }
        };

    if !output.status.success() {
        tracing::warn!("nmcli exited with error: {:?}", output.status);
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    tracing::info!("📡 nmcli output received ({} bytes)", text.len());

    for line in text.lines() {
        if line.starts_with("yes:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 5 {
                let ssid = if parts[1].is_empty() { None } else { Some(parts[1].to_string()) };
                let bssid = if parts[2].is_empty() { None } else { Some(parts[2].replace("\\", ":").to_string()) };
                let signal = parts[3].parse::<i32>().ok();
                let interface = parts[4].to_string();

                tracing::info!("✅ Connected to WiFi: SSID={:?}, Interface={}", ssid, interface);

                return Ok(Some(WifiSnapshot {
                    interface_name: interface,
                    state: "connected".to_string(),
                    ssid,
                    bssid,
                    signal_percent: signal,
                }));
            }
        }
    }

    tracing::info!("ℹ️ No active WiFi connection detected by nmcli");
    Ok(None)
}

#[cfg(target_os = "macos")]
fn current_wifi_snapshot() -> Result<Option<WifiSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    let output = match Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
        .arg("-I")
        .output() {
            Ok(out) => out,
            Err(e) => {
                tracing::warn!("Failed to execute airport command: {}", e);
                return Ok(None);
            }
        };

    if !output.status.success() {
        tracing::warn!("airport command exited with error: {:?}", output.status);
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut ssid: Option<String> = None;
    let mut bssid: Option<String> = None;
    let mut signal_percent: Option<i32> = None;
    let mut state = "disconnected".to_string();

    for line in text.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "SSID" => ssid = Some(value.to_string()),
                "BSSID" => bssid = Some(value.to_string()),
                "state" => {
                    if value == "running" {
                        state = "connected".to_string();
                    }
                }
                "agrCtlRSSI" => {
                    // RSSI is negative, usually -30 (good) to -90 (bad)
                    // Let's approximate a percentage
                    if let Ok(rssi) = value.parse::<f32>() {
                        let quality = 2.0 * (rssi + 100.0);
                        signal_percent = Some(quality.clamp(0.0, 100.0) as i32);
                    }
                }
                _ => {}
            }
        }
    }

    if state == "connected" && ssid.is_some() {
        tracing::info!("✅ Connected to WiFi: SSID={:?}", ssid);
        Ok(Some(WifiSnapshot {
            interface_name: "en0".to_string(), // Default on macOS
            state,
            ssid,
            bssid,
            signal_percent,
        }))
    } else {
        tracing::info!("ℹ️ No active WiFi connection detected by airport");
        Ok(None)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn current_wifi_snapshot() -> Result<Option<WifiSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(None)
}

fn parse_percent(value: &str) -> Option<i32> {
    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<i32>().ok()
}
