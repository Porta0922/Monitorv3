//! Desktop minimization at durable storage and transport boundaries.
use crate::config_manager::AgentConfig;
use serde_json::{json, Value};

fn excluded(app: &Value, config: &AgentConfig) -> bool {
    let name = app.as_str().unwrap_or("").to_lowercase();
    config.privacy_excluded_apps.iter().any(|item| name == item.trim().to_lowercase())
}

fn redact_title(value: &mut Value, config: &AgentConfig) {
    let title = value.as_str().unwrap_or("").to_lowercase();
    if config.privacy_hide_titles || config.privacy_title_filters.iter()
        .any(|term| title.contains(&term.trim().to_lowercase())) {
        *value = json!("[Título protegido]");
    }
}

pub fn apply(event: &mut Value, config: &AgentConfig) {
    if !event["payload"].is_object() { return; }
    match event["event_type"].as_str().unwrap_or("") {
        "activity" => {
            if excluded(&event["payload"]["app_name"], config) {
                event["payload"]["app_name"] = json!("[Aplicación excluida]");
                event["payload"]["window_title"] = json!("[Título protegido]");
            } else { redact_title(&mut event["payload"]["window_title"], config); }
        }
        "running_apps" => {
            if let Some(apps) = event["payload"]["apps"].as_array_mut() {
                apps.retain(|app| app.is_object() && !excluded(&app["app_name"], config));
                for app in apps {
                    redact_title(&mut app["primary_title"], config);
                    if config.privacy_hide_paths { app["exe_path"] = Value::Null; }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    #[test]
    fn privacy_hide_all_and_invalid_snapshot_entries_are_safe() {
        let config=AgentConfig {privacy_hide_titles:true, ..Default::default()};
        let mut event=json!({"event_type":"running_apps","payload":{"apps":[null,17,{"app_name":"editor","primary_title":"private"}]}});
        apply(&mut event,&config);
        assert_eq!(event["payload"]["apps"].as_array().unwrap().len(),1);
        assert_eq!(event["payload"]["apps"][0]["primary_title"],"[Título protegido]");
        let mut policy=config;
        policy.privacy_title_filters=vec![" ".into()];
        assert!(policy.validate().is_err());
        policy.privacy_title_filters=vec!["a".repeat(257)];
        assert!(policy.validate().is_err());
    }
    #[test]
    fn privacy_preserves_duration_identity_and_redacts_desktop_metadata() {
        let config = AgentConfig { privacy_hide_paths: true, privacy_excluded_apps: vec!["secret.exe".into()],
            privacy_title_filters: vec!["confidencial".into()], ..Default::default() };
        let mut event = json!({"event_id":"stable", "event_type":"activity", "payload":{
            "app_name":"SECRET.EXE", "window_title":"private", "duration_seconds":30}});
        apply(&mut event, &config);
        assert_eq!(event["payload"]["app_name"], "[Aplicación excluida]");
        assert_eq!(event["payload"]["duration_seconds"], 30);
        assert_eq!(event["event_id"], "stable");
        let mut snapshot = json!({"event_type":"running_apps", "payload":{"apps":[
            {"app_name":"secret.exe","primary_title":"private"},
            {"app_name":"editor","primary_title":"CONFIDENCIAL: cliente","exe_path":"/home/user/editor"}]}});
        apply(&mut snapshot, &config);
        assert_eq!(snapshot["payload"]["apps"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["payload"]["apps"][0]["primary_title"], "[Título protegido]");
        assert!(snapshot["payload"]["apps"][0]["exe_path"].is_null());
        let once = snapshot.clone(); apply(&mut snapshot, &config); assert_eq!(once, snapshot);
    }
}
