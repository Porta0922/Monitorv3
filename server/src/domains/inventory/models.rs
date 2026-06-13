use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub device_id: String,
    pub app_name: String,
    pub version: String,
    pub exe_hash: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAppItem {
    pub id: Uuid,
    pub device_id: Uuid,
    pub app_name: String,
    pub primary_title: String,
    pub window_count: i32,
    pub exe_path: Option<String>,
    pub exe_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopApp {
    pub app_name: String,
    pub total_duration_seconds: i64,
}
