use chrono::{DateTime, Utc};
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
    last_published_at: Option<DateTime<Utc>>,
}

impl WifiMonitor {
    pub fn new() -> Self {
        Self {
            last_key: None,
            last_published_at: None,
        }
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

        let changed = self.last_key.as_ref() != Some(&current_key);
        let periodic = self
            .last_published_at
            .map(|last| (Utc::now() - last).num_seconds() >= 600)
            .unwrap_or(true);

        if changed || periodic {
            self.last_key = Some(current_key);
            self.last_published_at = Some(Utc::now());
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
    let output = Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .output()?;

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
