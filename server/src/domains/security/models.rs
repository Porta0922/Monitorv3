use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub device_id: Uuid,
    pub query_name: String,
    pub query_pack: Option<String>,
    pub mitre_technique: Option<String>,
    pub severity: String,
    pub raw_data: serde_json::Value,
    pub event_fingerprint: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    pub id: i64,
    pub device_id: Uuid,
    pub alert_type: String,
    pub app_name: String,
    pub exe_hash: String,
    pub description: String,
    pub severity: String,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySummaryRow {
    pub severity: String,
    pub mitre_technique: String,
    pub event_count: i64,
}
