use chrono::{DateTime, Utc, Duration};
use serde::Deserialize;
use serde_json::json;
use crate::config::RuntimeConfig;
use crate::domains::device::models::Device;

#[derive(Debug, Deserialize, Default)]
pub struct ActivityLogFilters {
    pub limit: Option<i64>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub hours: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ActiveIdleQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LiveDevicesQuery {
    pub live_only: Option<bool>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TzQuery {
    pub tz_offset_minutes: Option<i32>,
}

pub fn format_duration(seconds: i64) -> String {
    let safe_seconds = seconds.max(0);
    let hours = safe_seconds / 3600;
    let minutes = (safe_seconds % 3600) / 60;
    let rem_seconds = safe_seconds % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, rem_seconds)
    } else {
        format!("{}s", rem_seconds)
    }
}

pub fn parse_time_bounds(filters: &ActivityLogFilters) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let from = filters
        .from
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));

    let to = filters
        .to
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));

    if from.is_none() {
        if let Some(hours) = filters.hours {
            if hours > 0 {
                return (Some(Utc::now() - Duration::hours(hours)), to);
            }
        }
    }
    
    (from, to)
}

pub fn serialize_device(device: Device, config: &RuntimeConfig) -> serde_json::Value {
    let online = device.last_seen > Utc::now() - Duration::seconds(config.online_threshold_seconds.max(1) as i64);
    let stale = !online;

    json!({
        "id": device.id,
        "device_id": device.device_id,
        "hostname": device.hostname,
        "nickname": device.nickname,
        "mac_address": device.mac_address.unwrap_or_else(|| "Unknown".to_string()),
        "created_at": device.created_at.to_rfc3339(),
        "last_seen": device.last_seen.to_rfc3339(),
        "online": online,
        "stale": stale,
        "status": if online { "online" } else { "offline" }
    })
}
