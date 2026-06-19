use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, watch};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            inventory_interval_days: 30,
            heatmap_interval_secs: 3600,
            resource_logger_interval_secs: 60,
            osquery_scheduler_seconds: 0,
            idle_threshold_seconds: 300,
            activity_heartbeat_seconds: 30,
            enabled_monitors: Vec::new(),
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
            inventory_interval_days: Self::env_u64("AGENT_INVENTORY_INTERVAL_DAYS", 30),
            heatmap_interval_secs: Self::env_u64("AGENT_HEATMAP_INTERVAL", 3600),
            resource_logger_interval_secs: Self::env_u64("AGENT_RESOURCE_LOGGER_INTERVAL", 60),
            osquery_scheduler_seconds: Self::env_u64("AGENT_OSQUERY_SCHEDULER_SECONDS", 0),
            idle_threshold_seconds: Self::env_u64("AGENT_IDLE_THRESHOLD", 300),
            activity_heartbeat_seconds: Self::env_u64("AGENT_ACTIVITY_HEARTBEAT", 30),
            enabled_monitors: Vec::new(),
        }
    }

    fn env_u64(key: &str, default: u64) -> u64 {
        std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn merge(&mut self, policy: Self) {
        if policy.window_activity_interval_secs != Self::default().window_activity_interval_secs {
            self.window_activity_interval_secs = policy.window_activity_interval_secs;
        }
        if policy.heartbeat_interval_secs != Self::default().heartbeat_interval_secs {
            self.heartbeat_interval_secs = policy.heartbeat_interval_secs;
        }
        if policy.usb_detector_interval_secs != Self::default().usb_detector_interval_secs {
            self.usb_detector_interval_secs = policy.usb_detector_interval_secs;
        }
        if policy.usb_copy_interval_secs != Self::default().usb_copy_interval_secs {
            self.usb_copy_interval_secs = policy.usb_copy_interval_secs;
        }
        if policy.wifi_interval_secs != Self::default().wifi_interval_secs {
            self.wifi_interval_secs = policy.wifi_interval_secs;
        }
        if policy.running_apps_interval_secs != Self::default().running_apps_interval_secs {
            self.running_apps_interval_secs = policy.running_apps_interval_secs;
        }
        if policy.inventory_interval_days != Self::default().inventory_interval_days {
            self.inventory_interval_days = policy.inventory_interval_days;
        }
        if policy.heatmap_interval_secs != Self::default().heatmap_interval_secs {
            self.heatmap_interval_secs = policy.heatmap_interval_secs;
        }
        if policy.resource_logger_interval_secs != Self::default().resource_logger_interval_secs {
            self.resource_logger_interval_secs = policy.resource_logger_interval_secs;
        }
        if policy.osquery_scheduler_seconds != Self::default().osquery_scheduler_seconds {
            self.osquery_scheduler_seconds = policy.osquery_scheduler_seconds;
        }
        if policy.idle_threshold_seconds != Self::default().idle_threshold_seconds {
            self.idle_threshold_seconds = policy.idle_threshold_seconds;
        }
        if policy.activity_heartbeat_seconds != Self::default().activity_heartbeat_seconds {
            self.activity_heartbeat_seconds = policy.activity_heartbeat_seconds;
        }
        if !policy.enabled_monitors.is_empty() {
            self.enabled_monitors = policy.enabled_monitors;
        }
    }

    pub fn is_monitor_enabled(&self, name: &str) -> bool {
        self.enabled_monitors.is_empty() || self.enabled_monitors.iter().any(|m| m == name)
    }
}

pub struct ConfigManager {
    inner: RwLock<AgentConfig>,
    version: AtomicU64,
    watch_tx: watch::Sender<AgentConfig>,
    #[allow(dead_code)]
    watch_rx: watch::Receiver<AgentConfig>,
}

impl ConfigManager {
    pub fn new() -> Arc<Self> {
        let config = AgentConfig::from_env();
        let (tx, rx) = watch::channel(config.clone());
        Arc::new(Self {
            inner: RwLock::new(config),
            version: AtomicU64::new(0),
            watch_tx: tx,
            watch_rx: rx,
        })
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

    pub async fn apply_policy(&self, policy: AgentConfig) {
        let mut config = self.inner.write().await;
        config.merge(policy);
        self.version.fetch_add(1, Ordering::Relaxed);
        let _ = self.watch_tx.send(config.clone());
        tracing::info!("[ConfigManager] Policy applied, version {}", self.version.load(Ordering::Relaxed));
    }

    pub async fn apply_policy_json(&self, json: serde_json::Value) {
        if let Ok(policy) = serde_json::from_value::<AgentConfig>(json) {
            self.apply_policy(policy).await;
        }
    }
}
