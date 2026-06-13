use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub interface_name: String,
    pub state: String,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub signal_percent: Option<i32>,
    pub timestamp: DateTime<Utc>,
}
