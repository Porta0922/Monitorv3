use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

// Coupled to the 32-day lower bound in history-retention.sql. Do not increase
// this horizon independently: markers removed under an earlier policy cannot be restored.
pub const MAX_REPLAY_DAYS: i64 = 30;
pub fn validate(event: &Value) -> Result<(), &'static str> {
    let observed = event["timestamp"].as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .ok_or("Telemetry requires a valid RFC3339 timestamp")?;
    let now = Utc::now();
    if observed < now - Duration::days(MAX_REPLAY_DAYS) {
        return Err("Telemetry exceeds the 30-day replay horizon");
    }
    if observed > now + Duration::seconds(120) {
        return Err("Telemetry timestamp is more than 120 seconds in the future");
    }
    Ok(())
}
