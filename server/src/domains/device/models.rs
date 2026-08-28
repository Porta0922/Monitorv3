use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTimeTotals {
    pub device_id: Uuid,
    pub active_seconds: i64,
    pub idle_seconds: i64,
    pub keys_count: i64,
    pub mouse_moves_count: i64,
    pub clicks_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveDeviceActivity {
    pub device_id: Uuid,
    pub app_name: String,
    pub window_title: String,
    pub duration_seconds: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub hostname: String,
    pub device_id: Uuid,
    pub mac_address: Option<String>,
    pub version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub devices_today: i64,
    pub active_time: i64,
    pub idle_time: i64,
    pub idle_pct: f64,
    pub keys_today: i64,
    pub mouse_moves_today: i64,
    pub mouse_clicks_today: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: i64,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalMetrics {
    pub devices_total: i64,
    pub devices_online: i64,
    pub devices_stale: i64,
    pub activities_last_hour: i64,
    pub input_rows_today: i64,
    pub newest_activity_age_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResourceMetric {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f64,
    pub memory_used_mb: f64,
    pub memory_percent: f64,
    pub top_process_name: Option<String>,
    pub top_process_cpu_percent: Option<f64>,
    pub top_process_memory_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResourcePeak {
    pub device_id: Uuid,
    pub peak_cpu_percent: f64,
    pub peak_memory_percent: f64,
    pub last_cpu_percent: f64,
    pub last_memory_percent: f64,
    pub top_process_name: Option<String>,
    pub top_process_cpu_percent: Option<f64>,
    pub top_process_memory_mb: Option<f64>,
    pub last_seen: DateTime<Utc>,
}
