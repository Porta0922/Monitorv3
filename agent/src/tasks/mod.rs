use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

pub mod window_activity;
pub mod heartbeat;
pub mod usb_detector;
pub mod usb_copy;
pub mod wifi_history;
pub mod running_apps;
pub mod inventory;
pub mod heatmap;
pub mod resource_logger;
pub mod session_input;
pub mod security_osquery;
pub mod support;

pub type SharedPublisher = Arc<RwLock<Option<Arc<crate::rabbitmq_publisher::RabbitMQPublisher>>>>;

pub struct EventMetadata {
    boot_id: String,
    sequence: AtomicU64,
}

impl EventMetadata {
    pub fn new() -> Self {
        Self {
            boot_id: Uuid::new_v4().to_string(),
            sequence: AtomicU64::new(1),
        }
    }

    pub fn next(&self) -> (String, u64, String) {
        let event_id = Uuid::new_v4().to_string();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        (event_id, sequence, self.boot_id.clone())
    }
}

pub struct TaskContext {
    pub device_id: String,
    pub hostname: String,
    pub mac_address: String,
    pub auth_token: String,
    pub publisher: SharedPublisher,
    pub cache: Arc<crate::offline_cache::OfflineCache>,
    pub keystroke_tracker: Arc<crate::keystroke_tracker::KeystrokeTracker>,
    pub input_tracker: Arc<crate::input_tracking::InputTracker>,
    pub envelope_metadata: Arc<EventMetadata>,
    pub wifi_resend_flag: Arc<AtomicBool>,
    pub config_manager: Arc<crate::config_manager::ConfigManager>,
    pub events_counter: Option<Arc<std::sync::atomic::AtomicU64>>,
}

impl TaskContext {
    pub fn build_event_envelope(
        &self,
        event_type: &str,
        schema_version: u32,
        payload: serde_json::Value,
    ) -> serde_json::Value {
        let (event_id, sequence, boot_id) = self.envelope_metadata.next();

        serde_json::json!({
            "event_id": event_id,
            "sequence": sequence,
            "boot_id": boot_id,
            "schema_version": schema_version,
            "event_type": event_type,
            "source_role": if crate::runtime::is_session() { "session" } else if crate::is_running_in_session_0() { "service" } else { "interactive" },
            "device_id": self.device_id,
            "hostname": self.hostname,
            "mac_address": self.mac_address,
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": Utc::now().to_rfc3339(),
            "payload": payload,
        })
    }

    pub async fn publish_or_cache(&self, routing_event_type: &str, payload: serde_json::Value) -> bool {
        // Recheck policy at the write boundary; a collector may have been in flight when paused.
        let config = self.config_manager.get().await;
        let monitor = match routing_event_type {
            "activity" => Some("window_activity"), "running_apps" => Some("running_apps"),
            "input_heatmaps" => Some("heatmap"), "inventory" => Some("inventory"),
            "usb" => Some("usb_detector"), "wifi" => Some("wifi_history"),
            "heartbeat" if payload["event_type"] == "input_summary" =>
                Some(if crate::runtime::is_session() { "session_input" } else { "resource_logger" }),
            "heartbeat" => Some("heartbeat"), _ => None,
        };
        if monitor.is_some_and(|name| !config.is_monitor_enabled(name))
            || (routing_event_type == "security" && config.monitoring_paused) { return false; }
        // Persist before attempting any transport. A failed write is observable and never acknowledged.
        let cache_id = match self.cache.save_event(routing_event_type, &payload).await {
            Ok(id) => id,
            Err(error) => {
                tracing::error!(%error, route = routing_event_type, "Event could not be queued; disk requires attention");
                return false;
            }
        };
        if !crate::runtime::is_session() {
            let publisher_snapshot = { self.publisher.read().await.clone() };
            if let Some(publisher) = publisher_snapshot {
                match publisher.publish_event(routing_event_type, payload).await {
                    Ok(()) => {
                        if let Err(error) = self.cache.mark_synced(&[cache_id]).await {
                            tracing::error!(%error, "Cannot persist publish confirmation; event may be retried");
                        }
                    }
                    Err(error) => tracing::warn!(%error, "Publish failed; event remains queued"),
                }
            }
        }

        if let Some(counter) = &self.events_counter {
            counter.fetch_add(1, Ordering::Relaxed);
        }
        true
    }
}

pub fn skip_interval(duration: std::time::Duration) -> tokio::time::Interval {
    let mut int = tokio::time::interval(duration);
    int.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    int
}

#[derive(Clone, Copy)]
pub enum TaskInterval {
    WindowActivity,
    Heartbeat,
    UsbDetector,
    UsbCopy,
    Wifi,
    RunningApps,
    Inventory,
    Heatmap,
    ResourceLogger,
    Osquery,
}

impl TaskInterval {
    /// Effective tick seconds for this task reading the LIVE config.
    fn secs(&self, config: &crate::config_manager::AgentConfig) -> u64 {
        match self {
            TaskInterval::WindowActivity => config.window_activity_interval_secs.clamp(2, 60),
            TaskInterval::Heartbeat => config.heartbeat_interval_secs.max(1),
            TaskInterval::UsbDetector => config.usb_detector_interval_secs.max(1),
            TaskInterval::UsbCopy => config.usb_copy_interval_secs.max(1),
            TaskInterval::Wifi => config.wifi_interval_secs.max(1),
            TaskInterval::RunningApps => config.running_apps_interval_secs.max(1),
            TaskInterval::Inventory => (config.inventory_interval_days as u64 * 86400).max(1),
            TaskInterval::Heatmap => config.heatmap_interval_secs.max(1),
            TaskInterval::ResourceLogger => config.resource_logger_interval_secs.max(1),
            TaskInterval::Osquery => (config.osquery_scheduler_seconds as u64).max(60),
        }
    }
}

/// Sleeps for the task's configured interval, re-reading the live config on
/// every cycle so policy changes take effect without restarting the agent.
pub async fn live_sleep(context: &TaskContext, interval: TaskInterval) {
    let mut changes = context.config_manager.subscribe();
    loop {
        let config = changes.borrow_and_update().clone();
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(interval.secs(&config))) => return,
            changed = changes.changed() => { if changed.is_err() { return; } },
        }
    }
}
