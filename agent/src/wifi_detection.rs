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
            // Disable periodic broadcast by setting a very high interval (once every 24 hours)
            periodic_broadcast_interval: 1440, 
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

        // Detect significant signal drop (>= 20 percentage points)
        let signal_dropped = match (self.last_signal, snapshot.signal_percent) {
            (Some(last), Some(current)) => last - current >= 20,
            _ => false,
        };

        self.scans_since_broadcast += 1;
        let periodic_due = self.scans_since_broadcast >= self.periodic_broadcast_interval;

        if state_changed || signal_dropped || periodic_due {
            self.last_key = Some(current_key);
            self.last_signal = snapshot.signal_percent;
            self.scans_since_broadcast = 0;
            return Ok(Some(snapshot));
        }

        // Always update tracked signal so drops are measured from current level
        self.last_signal = snapshot.signal_percent;
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

#[cfg(not(target_os = "windows"))]
fn current_wifi_snapshot() -> Result<Option<WifiSnapshot>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(None)
}

fn parse_percent(value: &str) -> Option<i32> {
    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<i32>().ok()
}
