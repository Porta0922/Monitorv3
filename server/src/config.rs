#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub online_threshold_seconds: i64,
    pub live_threshold_seconds: i64,
    pub stale_threshold_seconds: i64,
    pub input_bucket_max_seconds: i64,
    pub max_event_past_skew_seconds: i64,
    pub max_event_future_skew_seconds: i64,
    pub readiness_timeout_ms: u64,
    pub stream_poll_interval_ms: u64,
    pub stream_fetch_limit: i64,
    pub stream_max_devices: usize,
    pub audit_log_enabled: bool,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            online_threshold_seconds: env_i64("ONLINE_THRESHOLD_SECONDS", 300),
            live_threshold_seconds: env_i64("LIVE_THRESHOLD_SECONDS", 180),
            stale_threshold_seconds: env_i64("STALE_THRESHOLD_SECONDS", 120),
            input_bucket_max_seconds: env_i64("INPUT_BUCKET_MAX_SECONDS", 60),
            max_event_past_skew_seconds: env_i64("MAX_EVENT_PAST_SKEW_SECONDS", 7200),
            max_event_future_skew_seconds: env_i64("MAX_EVENT_FUTURE_SKEW_SECONDS", 120),
            readiness_timeout_ms: env_u64("READINESS_TIMEOUT_MS", 1500),
            stream_poll_interval_ms: env_u64("STREAM_POLL_INTERVAL_MS", 2000),
            stream_fetch_limit: env_i64("STREAM_FETCH_LIMIT", 500),
            stream_max_devices: env_usize("STREAM_MAX_DEVICES", 100),
            audit_log_enabled: env_bool("AUDIT_LOG_ENABLED", true),
        }
    }
}

fn env_i64(key: &str, default_value: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default_value)
}

fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_value)
}

fn env_usize(key: &str, default_value: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default_value)
}

fn env_bool(key: &str, default_value: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default_value,
    }
}
