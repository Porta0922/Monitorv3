use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, watch};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub window_activity_interval_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub usb_detector_interval_secs: u64,
    pub usb_copy_interval_secs: u64,
    pub wifi_interval_secs: u64,
    pub running_apps_interval_secs: u64,
    pub inventory_interval_days: u64,
    pub heatmap_interval_secs: u64,
    pub resource_logger_interval_secs: u64,
    pub osquery_scheduler_seconds: u64,
    pub idle_threshold_seconds: u64,
    pub activity_heartbeat_seconds: u64,
    pub enabled_monitors: Vec<String>,
    #[serde(default)]
    pub disabled_monitors: Vec<String>,
    #[serde(default)]
    pub monitoring_paused: bool,
    #[serde(default)]
    pub privacy_hide_titles: bool,
    #[serde(default)]
    pub privacy_hide_paths: bool,
    #[serde(default)]
    pub privacy_excluded_apps: Vec<String>,
    #[serde(default)]
    pub privacy_title_filters: Vec<String>,

}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            window_activity_interval_secs: 2,
            heartbeat_interval_secs: 60,
            usb_detector_interval_secs: 60,
            usb_copy_interval_secs: 60,
            wifi_interval_secs: 120,
            running_apps_interval_secs: 60,
            inventory_interval_days: 1,
            heatmap_interval_secs: 3600,
            resource_logger_interval_secs: 60,
            osquery_scheduler_seconds: 0,
            idle_threshold_seconds: 300,
            activity_heartbeat_seconds: 30,
            enabled_monitors: Vec::new(),
            disabled_monitors: Vec::new(),
            monitoring_paused: false,
            privacy_hide_titles: false, privacy_hide_paths: false,
            privacy_excluded_apps: Vec::new(), privacy_title_filters: Vec::new(),
        }
    }
}

impl AgentConfig {
    pub fn from_env() -> Self {
        Self {
            window_activity_interval_secs: Self::env_u64("AGENT_WINDOW_ACTIVITY_INTERVAL", 2),
            heartbeat_interval_secs: Self::env_u64("AGENT_HEARTBEAT_INTERVAL", 60),
            usb_detector_interval_secs: Self::env_u64("AGENT_USB_DETECTOR_INTERVAL", 60),
            usb_copy_interval_secs: Self::env_u64("AGENT_USB_COPY_INTERVAL", 60),
            wifi_interval_secs: Self::env_u64("AGENT_WIFI_INTERVAL", 120),
            running_apps_interval_secs: Self::env_u64("AGENT_RUNNING_APPS_INTERVAL", 60),
            inventory_interval_days: Self::env_u64("AGENT_INVENTORY_INTERVAL_DAYS", 1),
            heatmap_interval_secs: Self::env_u64("AGENT_HEATMAP_INTERVAL", 3600),
            resource_logger_interval_secs: Self::env_u64("AGENT_RESOURCE_LOGGER_INTERVAL", 60),
            osquery_scheduler_seconds: Self::env_u64("AGENT_OSQUERY_SCHEDULER_SECONDS", 0),
            idle_threshold_seconds: Self::env_u64("AGENT_IDLE_THRESHOLD", 300),
            activity_heartbeat_seconds: Self::env_u64("AGENT_ACTIVITY_HEARTBEAT", 30),
            enabled_monitors: Vec::new(),
            disabled_monitors: Vec::new(),
            monitoring_paused: false,
            privacy_hide_titles: false, privacy_hide_paths: false,
            privacy_excluded_apps: Vec::new(), privacy_title_filters: Vec::new(),
        }
    }

    fn env_u64(key: &str, default: u64) -> u64 {
        std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn validate(&self) -> Result<(), String> {
        for list in [&self.privacy_excluded_apps, &self.privacy_title_filters] {
            if list.len() > 100 || list.iter().any(|v| v.trim().is_empty() || v.len() > 256 || v.chars().any(char::is_control)) {
                return Err("Invalid desktop privacy rules".into());
            }
        }
        for (name, value, minimum, maximum) in [
            ("window_activity_interval_secs", self.window_activity_interval_secs, 2, 60),
            ("heartbeat_interval_secs", self.heartbeat_interval_secs, 1, 86400),
            ("usb_detector_interval_secs", self.usb_detector_interval_secs, 1, 86400),
            ("usb_copy_interval_secs", self.usb_copy_interval_secs, 1, 86400),
            ("wifi_interval_secs", self.wifi_interval_secs, 1, 86400),
            ("running_apps_interval_secs", self.running_apps_interval_secs, 1, 86400),
            ("inventory_interval_days", self.inventory_interval_days, 1, 365),
            ("heatmap_interval_secs", self.heatmap_interval_secs, 1, 86400),
            ("resource_logger_interval_secs", self.resource_logger_interval_secs, 1, 60),
            ("osquery_scheduler_seconds", self.osquery_scheduler_seconds, 0, 86400),
            ("idle_threshold_seconds", self.idle_threshold_seconds, 1, 86400),
            ("activity_heartbeat_seconds", self.activity_heartbeat_seconds, 1, 3600),
        ] {
            if !(minimum..=maximum).contains(&value) { return Err(format!("{name} is out of range")); }
        }
        const MONITORS: &[&str] = &["window_activity", "heartbeat", "usb_detector", "usb_copy", "wifi_history",
            "running_apps", "inventory", "heatmap", "resource_logger", "security_osquery", "session_input"];
        if self.enabled_monitors.iter().chain(&self.disabled_monitors).any(|m| !MONITORS.contains(&m.as_str())) {
            return Err("Unknown monitor in policy".into());
        }
        let mut names=std::collections::HashSet::new();
        if self.enabled_monitors.iter().chain(&self.disabled_monitors).any(|name|!names.insert(name)) {return Err("Duplicate or conflicting monitor policy".into());}
        Ok(())
    }

    pub fn is_monitor_enabled(&self, name: &str) -> bool {
        if self.monitoring_paused || self.disabled_monitors.iter().any(|m| m == name) { return false; }
        // Existing resource_logger policies also control its session-only counter component.
        if name == "session_input" {
            return !self.disabled_monitors.iter().any(|m| m == "resource_logger") &&
                (self.enabled_monitors.is_empty() || self.enabled_monitors.iter().any(|m| m == name || m == "resource_logger"));
        }
        self.enabled_monitors.is_empty() || self.enabled_monitors.iter().any(|m| m == name)
    }

}

pub struct ConfigManager {
    path: std::path::PathBuf,
    inner: RwLock<AgentConfig>,
    version: AtomicU64,
    watch_tx: watch::Sender<AgentConfig>,
    #[allow(dead_code)]
    watch_rx: watch::Receiver<AgentConfig>,
}

impl ConfigManager {
    pub fn new() -> std::io::Result<Arc<Self>> {
        Self::from_path(crate::runtime::data_dir().join("effective_policy.json"))
    }

    pub fn from_path(path: std::path::PathBuf) -> std::io::Result<Arc<Self>> {
        let config: AgentConfig = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(std::io::Error::other)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => AgentConfig::from_env(),
            Err(error) => return Err(error),
        };
        config.validate().map_err(std::io::Error::other)?;
        let (tx, rx) = watch::channel(config.clone());
        Ok(Arc::new(Self {
            path,
            inner: RwLock::new(config),
            version: AtomicU64::new(0),
            watch_tx: tx,
            watch_rx: rx,
        }))
    }

    pub async fn get(&self) -> AgentConfig {
        self.inner.read().await.clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<AgentConfig> {
        self.watch_tx.subscribe()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    /// A complete authoritative snapshot, including default values and empty lists.
    pub async fn apply_policy(&self, policy: AgentConfig) -> std::io::Result<()> {
        policy.validate().map_err(std::io::Error::other)?;
        let mut config = self.inner.write().await;
        if *config == policy && self.path.is_file() { return Ok(()); }
        crate::persistent_file::write_json(&self.path, &policy)?;
        *config = policy;
        self.version.fetch_add(1, Ordering::Relaxed);
        self.watch_tx.send_replace(config.clone());
        tracing::info!("Policy snapshot applied, local revision {}", self.version());
        Ok(())
    }

    pub async fn apply_policy_json(&self, json: serde_json::Value) -> std::io::Result<()> {
        self.apply_policy(serde_json::from_value(json).map_err(std::io::Error::other)?).await
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    #[tokio::test]
    async fn policy_persists_pause_and_resets_to_defaults_without_partial_apply() {
        let dir = std::env::temp_dir().join(format!("monitor-policy-{}", uuid::Uuid::new_v4()));
        let path = dir.join("policy.json");
        let manager = ConfigManager::from_path(path.clone()).unwrap();
        let mut policy = AgentConfig::default();
        policy.monitoring_paused = true;
        manager.apply_policy(policy).await.unwrap();
        assert!(!manager.get().await.is_monitor_enabled("heartbeat"));
        let restored = ConfigManager::from_path(path.clone()).unwrap();
        assert!(restored.get().await.monitoring_paused);
        restored.apply_policy(AgentConfig::default()).await.unwrap();
        assert!(restored.get().await.is_monitor_enabled("heartbeat"));
        let mut invalid = AgentConfig::default(); invalid.resource_logger_interval_secs = 0;
        assert!(restored.apply_policy(invalid).await.is_err());
        assert_eq!(restored.get().await, AgentConfig::default());
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(ConfigManager::from_path(path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn disabled_session_input_alias_wins_and_unknown_collectors_are_rejected() {
        let mut policy = AgentConfig::default();
        policy.enabled_monitors = vec!["resource_logger".into()];
        assert!(policy.is_monitor_enabled("session_input"));
        policy.disabled_monitors = vec!["resource_logger".into()];
        assert!(!policy.is_monitor_enabled("session_input"));
        policy.enabled_monitors.push("typo".into());
        assert!(policy.validate().is_err());
    }
}
