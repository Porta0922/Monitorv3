//! Latest capture result only; task liveness is a separate signal.
use std::{collections::BTreeMap, sync::{LazyLock, Mutex}};
use serde_json::{json, Value};
static RESULTS: LazyLock<Mutex<BTreeMap<&'static str, Value>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));

pub fn record(name: &'static str, state: &str, error: Option<&str>) {
    if !["usb_detector", "wifi_history", "inventory", "running_apps", "resource_logger", "window_activity", "usb_copy", "security_osquery", "heatmap", "heartbeat", "session_input"].contains(&name) { return; }
    let error = error.map(|s| s.chars().take(1000).collect::<String>());
    let mut results = RESULTS.lock().unwrap_or_else(|e| e.into_inner());
    results.insert(name, json!({"state":state,"last_error":error,"observed_at":chrono::Utc::now().to_rfc3339()}));
}
pub fn latest(name: &str) -> Value {
    RESULTS.lock().unwrap_or_else(|e| e.into_inner()).get(name).cloned()
        .unwrap_or_else(||json!({"state":"unknown","last_error":null,"observed_at":null}))
}
