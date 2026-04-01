// WebSocket types for real-time updates
// Place in server/src/ws.rs

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// WebSocket message types for real-time dashboard updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Device status changed (online/offline)
    #[serde(rename = "device_status")]
    DeviceStatus {
        device_id: String,
        online: bool,
        last_seen: String,
    },

    /// New activity log entry
    #[serde(rename = "activity_log")]
    ActivityLog {
        device_id: String,
        app_name: String,
        window_title: String,
        timestamp: String,
    },

    /// USB device connected/disconnected
    #[serde(rename = "usb_event")]
    UsbEvent {
        device_id: String,
        action: String, // "IN" or "OUT"
        hardware_id: String,
        device_name: String,
        timestamp: String,
    },

    /// New security alert
    #[serde(rename = "security_alert")]
    SecurityAlert {
        device_id: String,
        alert_type: String,
        severity: String,
        app_name: String,
        description: String,
        timestamp: String,
    },

    /// Subscription confirmation
    #[serde(rename = "subscribed")]
    Subscribed {
        message: String,
    },

    /// Ping/heartbeat
    #[serde(rename = "ping")]
    Ping,

    /// Pong/heartbeat response
    #[serde(rename = "pong")]
    Pong,
}

/// WebSocket subscription manager
pub struct WsSubscriber {
    device_ids: Arc<RwLock<HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<WsMessage>>>>>,
}

impl WsSubscriber {
    /// Create new subscriber
    pub fn new() -> Self {
        Self {
            device_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to device updates
    pub async fn subscribe(&self, device_id: String) -> tokio::sync::mpsc::UnboundedReceiver<WsMessage> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut subscriptions = self.device_ids.write().await;
        subscriptions.entry(device_id).or_insert_with(Vec::new).push(tx);
        rx
    }

    /// Broadcast message to all subscribers of a device
    pub async fn broadcast(&self, device_id: &str, message: WsMessage) {
        let subscriptions = self.device_ids.read().await;
        if let Some(senders) = subscriptions.get(device_id) {
            for sender in senders {
                let _ = sender.send(message.clone());
            }
        }
    }

    /// Broadcast to all devices (for server status updates)
    pub async fn broadcast_all(&self, message: WsMessage) {
        let subscriptions = self.device_ids.read().await;
        for senders in subscriptions.values() {
            for sender in senders {
                let _ = sender.send(message.clone());
            }
        }
    }
}

impl Clone for WsSubscriber {
    fn clone(&self) -> Self {
        Self {
            device_ids: Arc::clone(&self.device_ids),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_subscriber() {
        let subscriber = WsSubscriber::new();
        let mut rx = subscriber.subscribe("device-1".to_string()).await;

        let msg = WsMessage::Ping;
        subscriber.broadcast("device-1", msg.clone()).await;

        assert!(rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_broadcast_all() {
        let subscriber = WsSubscriber::new();
        let mut rx1 = subscriber.subscribe("device-1".to_string()).await;
        let mut rx2 = subscriber.subscribe("device-2".to_string()).await;

        let msg = WsMessage::Ping;
        subscriber.broadcast_all(msg.clone()).await;

        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
    }
}
