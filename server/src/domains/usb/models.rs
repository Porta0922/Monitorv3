use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub action: String,
    pub hardware_id: String,
    pub device_name: String,
    pub serial_number: String,
    pub volume_label: Option<String>,
    pub timestamp: DateTime<Utc>,
}
