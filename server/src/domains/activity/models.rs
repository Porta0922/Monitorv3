use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: Uuid,
    pub device_id: Uuid,
    pub app_name: String,
    pub window_title: String,
    pub duration_seconds: i64,
    pub timestamp: DateTime<Utc>,
}
