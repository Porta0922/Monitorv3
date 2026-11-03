//! Validation of current-state telemetry before it enters the dashboard contract.
use serde_json::Value;
fn number(v: &Value, max: f64) -> bool {
    v.is_null() || v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0 && n <= max)
}
fn text(v: &Value, max: usize) -> bool { v.as_str().is_some_and(|s|s.len()<=max) }
fn capabilities(v: &Value) -> bool {
    v.is_object() && ["mode","os","architecture","desktop"].iter().all(|key| v[*key].is_null() || text(&v[*key],128))
        && ["input_available","desktop_capture_eligible"].iter().all(|key|v[*key].is_null() || v[*key].is_boolean())
}
pub fn resources(v: &Value) -> Result<(), &'static str> {
    if v["schema_version"].as_u64()!=Some(1) || v.to_string().len()>131072 || !capabilities(&v["capabilities"]) {
        return Err("Invalid metrics version, size or capabilities");
    }
    for key in ["cpu_percent","memory_percent"] { if !number(&v[key],100.0) { return Err("Invalid percentage"); } }
    for key in ["memory_used_bytes","memory_total_bytes","uptime_seconds","sample_interval_seconds"] {
        if !number(&v[key],u64::MAX as f64) { return Err("Invalid resource quantity"); }
    }
    for key in ["cpu_state","memory_state","disks_state","network_state"] {
        if !matches!(v[key].as_str(),Some("available"|"unavailable"|"warming_up")) { return Err("Invalid resource state"); }
    }
    if let Some(hardware)=v.get("hardware").filter(|h|!h.is_null()) {
        if !hardware.is_object() || !number(&hardware["logical_cpu_count"],1048576.0)
            || ["cpu_model","os_version","kernel_version"].iter().any(|key|!(hardware[*key].is_null() || text(&hardware[*key],1024))) {
            return Err("Invalid hardware metadata");
        }
    }
    let disks=v["disks"].as_array().ok_or("Missing disks array")?;
    let network=v["network"].as_array().ok_or("Missing network array")?;
    if disks.len()>128 || network.len()>128 { return Err("Too many resource entries"); }
    if disks.iter().any(|d| !text(&d["mount"],4096) || !number(&d["total_bytes"],u64::MAX as f64)
        || !number(&d["available_bytes"],u64::MAX as f64) || !number(&d["used_percent"],100.0)) { return Err("Invalid disk entry"); }
    if network.iter().any(|n| !text(&n["name"],1024) || !number(&n["receive_bytes_per_second"],u64::MAX as f64)
        || !number(&n["transmit_bytes_per_second"],u64::MAX as f64)) { return Err("Invalid network entry"); }
    Ok(())
}
pub fn health(v: &Value) -> Result<(), &'static str> {
    if v.to_string().len()>131072 || !capabilities(&v["capabilities"]) || !matches!(v["status"].as_str(),Some("healthy"|"degraded")) {
        return Err("Invalid health report");
    }
    if !v["monitoring_paused"].is_boolean() || !number(&v["cache_unsynced_events"],u64::MAX as f64)
        || !(v["version"].is_null() || text(&v["version"],128)) { return Err("Invalid health metadata"); }
    if let Some(collectors) = v.get("collectors").filter(|c|!c.is_null()) {
        let items = collectors.as_array().ok_or("Invalid collectors")?;
        for collector in items {
            if let Some(capture) = collector.get("capture").filter(|c| !c.is_null()) {
                if !capture.is_object()
                    || !matches!(capture["state"].as_str(), Some("capture_ok"|"capture_error"|"no_observation"|"unknown"))
                    || !(capture["last_error"].is_null() || text(&capture["last_error"],4096))
                    || !(capture["observed_at"].is_null() || capture["observed_at"].as_str().is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())) {
                    return Err("Invalid capture diagnostics");
                }
            }
        }
        if items.len()>128 || items.iter().any(|c|!text(&c["name"],128) || !text(&c["state"],64)
            || !(c["last_error"].is_null() || text(&c["last_error"],4096))) { return Err("Invalid collector diagnostics"); }
    }
    if let Some(sessions)=v.get("sessions").filter(|s|!s.is_null()) {
        if !sessions["relay_enabled"].is_boolean() || !matches!(sessions["producer_mode"].as_str(),Some("single_lease"|"per_principal")) {return Err("Invalid session mode");}
        let instances=sessions["instances"].as_array().ok_or("Invalid session diagnostics")?;
        if instances.len()>16 {return Err("Too many session diagnostics");}
        for instance in instances {
            if !instance["instance_id"].as_str().is_some_and(|s|uuid::Uuid::parse_str(s).is_ok())
                || !instance["received_at"].as_str().is_some_and(|s|chrono::DateTime::parse_from_rfc3339(s).is_ok())
                || ["stale","monitoring_paused","input_available"].iter().any(|key|!instance[*key].is_boolean())
                || !(instance["pending_events"].is_null() || instance["pending_events"].as_u64().is_some())
                || !instance["policy_fingerprint"].as_str().is_some_and(|s|s.len()==64 && s.bytes().all(|b|b.is_ascii_hexdigit())) {return Err("Invalid session observation");}
            if !(instance["principal"].is_null() || text(&instance["principal"],128)) {return Err("Invalid session principal");}
            let tasks=instance["tasks"].as_array().ok_or("Invalid session tasks")?;
            for task in tasks {
                if let Some(c)=task.get("capture").filter(|c|!c.is_null()) {
                    if !matches!(c["state"].as_str(),Some("capture_ok"|"capture_error"|"no_observation"|"unknown"))
                        || !(c["last_error"].is_null() || text(&c["last_error"],1024))
                        || !(c["observed_at"].is_null() || c["observed_at"].as_str().is_some_and(|s|chrono::DateTime::parse_from_rfc3339(s).is_ok())) {return Err("Invalid session capture diagnostic");}
                }
            }
            if tasks.len()>16 || tasks.iter().any(|t|!text(&t["name"],128) || !t["running"].is_boolean()) {return Err("Invalid session tasks");}
        }
    }
    let tasks=v["tasks"].as_array().ok_or("Missing tasks array")?;
    if tasks.len()>128 || tasks.iter().any(|t| !text(&t["name"],128) || !t["running"].is_boolean()
        || t["restart_count"].as_u64().is_none() || !(t["last_error"].is_null() || text(&t["last_error"],4096))) {
        return Err("Invalid task diagnostics");
    }
    Ok(())
}
