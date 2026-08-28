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
            "device_id": self.device_id,
            "hostname": self.hostname,
            "mac_address": self.mac_address,
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": Utc::now().to_rfc3339(),
            "auth_token": self.auth_token,
            "payload": payload,
        })
    }

    pub async fn publish_or_cache(&self, routing_event_type: &str, payload: serde_json::Value) {
        let publisher_snapshot = { self.publisher.read().await.clone() };

        if let Some(pub_) = publisher_snapshot {
            let publish_error = match pub_.publish_event(routing_event_type, payload.clone()).await {
                Ok(_) => None,
                Err(err) => Some(err.to_string()),
            };

            if let Some(err_msg) = publish_error {
                tracing::warn!("Publish failed for {}. Caching event: {}", routing_event_type, err_msg);
                let _ = self.cache.save_event(routing_event_type, &payload).await;
            }
        } else {
            let _ = self.cache.save_event(routing_event_type, &payload).await;
        }

        if let Some(counter) = &self.events_counter {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn skip_interval(duration: std::time::Duration) -> tokio::time::Interval {
    let mut int = tokio::time::interval(duration);
    int.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    int
}
