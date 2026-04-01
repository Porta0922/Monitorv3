// Shared data structures for communication between agent and server

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ============ Device & Registration ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistration {
    pub device_id: Uuid,
    pub hostname: String,
    pub mac_address: String,
    pub os: String,  // "windows", "linux", "macos"
    pub agent_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: Uuid,
    pub hostname: String,
    pub nickname: Option<String>,
    pub mac_address: String,
    pub last_seen: DateTime<Utc>,
    pub online: bool,
}

// ============ Activity Logs ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub timestamp: DateTime<Utc>,
    pub device_id: Uuid,
    pub app_name: String,
    pub window_title: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogBatch {
    pub device_id: Uuid,
    pub events: Vec<ActivityEvent>,
    pub batch_id: String,
}

// ============ Software Inventory ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub app_name: String,
    pub version: Option<String>,
    pub exe_path: String,
    pub exe_hash: String,  // SHA-256
    pub installed_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryScan {
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub software: Vec<SoftwareInfo>,
}

// ============ Security & Hashing ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashVerification {
    pub app_name: String,
    pub exe_hash: String,
    pub expected_hash: Option<String>,
    pub is_verified: bool,
    pub alert_if_mismatch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    pub device_id: Uuid,
    pub alert_type: String,  // "HASH_MISMATCH", "UNKNOWN_EXECUTABLE", etc.
    pub app_name: String,
    pub exe_hash: String,
    pub description: String,
    pub severity: String,  // "LOW", "MEDIUM", "HIGH", "CRITICAL"
    pub timestamp: DateTime<Utc>,
}

// ============ API Requests/Responses ============

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub device_id: Uuid,
    pub hostname: String,
    pub mac_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

// ============ Offline Cache ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub synced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineEvent {
    pub device_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub retries: i32,
}
