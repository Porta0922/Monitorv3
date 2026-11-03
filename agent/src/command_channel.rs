use std::sync::Arc;
use tokio::time::Duration;
use crate::config_manager::ConfigManager;
use crate::task_supervisor::TaskSupervisor;
use crate::tasks::TaskContext;

pub async fn process_command(
    command: &str,
    _payload: Option<serde_json::Value>,
    context: &TaskContext,
    supervisor: &TaskSupervisor,
    config_manager: &ConfigManager,
) -> serde_json::Value {
    if !matches!(command,"ping"|"status"|"run_diagnostic") {return serde_json::json!({"status":"error","message":"Remote action disabled; use administrative lifecycle tools"});}
    match command {
        "ping" => {
            serde_json::json!({
                "status": "pong",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "version": env!("CARGO_PKG_VERSION"),
            })
        }
        "status" => {
            let handles = supervisor.handles();
            let tasks_status: Vec<serde_json::Value> = handles
                .iter()
                .map(|h| {
                    serde_json::json!({
                        "name": h.name,
                        "running": h.running,
                        "restart_count": h.restart_count,
                        "last_error": h.last_error,
                        "uptime_seconds": h.uptime_seconds,
                    })
                })
                .collect();
            serde_json::json!({
                "device_id": context.device_id,
                "hostname": context.hostname,
                "config_version": config_manager.version(),
                "healthy": handles.iter().all(|h| h.running),
                "tasks": tasks_status,
                "version": env!("CARGO_PKG_VERSION"),
            })
        }
        "run_diagnostic" => {
            let handles = supervisor.handles();
            let config = config_manager.get().await;
            let (cache_total, cache_unsynced) = context.cache.get_stats().await.unwrap_or((0, 0));
            serde_json::json!({
                "device_id": context.device_id,
                "hostname": context.hostname,
                "mac_address": context.mac_address,
                "version": env!("CARGO_PKG_VERSION"),
                "config_version": config_manager.version(),
                "config": config,
                "tasks": handles.iter().map(|h| serde_json::json!({
                    "name": h.name,
                    "running": h.running,
                    "restart_count": h.restart_count,
                    "last_error": h.last_error,
                    "uptime_seconds": h.uptime_seconds,
                })).collect::<Vec<_>>(),
                "cache": {
                    "total": cache_total,
                    "unsynced": cache_unsynced,
                },
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })
        }
        _ => {
            serde_json::json!({
                "status": "error",
                "message": format!("Unknown command: {}", command),
            })
        }
    }
}

pub fn spawn_command_channel(
    context: Arc<TaskContext>,
    supervisor: Arc<TaskSupervisor>,
    config_manager: Arc<ConfigManager>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let server_url = match std::env::var("AGENT_SERVER_URL") {
                Ok(url) => url.trim_end_matches('/').to_string(),
                Err(_) => continue,
            };

            let client = match crate::network_identity::client()
            {
                Ok(c) => c,
                Err(_) => continue,
            };

            let request = client
                .get(format!("{}/api/agent/commands", server_url))
                .query(&[("device_id", &context.device_id)])
                .header("x-agent-token", crate::network_identity::token(&context.auth_token));

            let response = match request.send().await {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let envelope: CommandsEnvelope = match response.json().await {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !envelope.success {
                continue;
            }

            for cmd in envelope.commands.unwrap_or_default() {
                if cmd.expires_at.as_deref().and_then(|v|chrono::DateTime::parse_from_rfc3339(v).ok()).is_none_or(|expiry|expiry<=chrono::Utc::now()) {continue;}
                tracing::info!("[CommandChannel] Received command: {} (id={})", cmd.command, cmd.id);
                let result = process_command(
                    &cmd.command,
                    cmd.payload,
                    &context,
                    &supervisor,
                    &config_manager,
                )
                .await;

                let ack_client = match crate::network_identity::client()
                {
                    Ok(c) => c,
                    Err(_) => break,
                };

                let ack_url = format!("{}/api/agent/commands/{}/ack", server_url, cmd.id);
                let _ = ack_client
                    .post(&ack_url)
                    .json(&serde_json::json!({
                        "device_id": context.device_id,
                        "status": if result["status"]=="error" || result["status"]=="disabled" {"failed"} else {"ok"},
                        "result": result,
                    }))
                    .header("x-agent-token", crate::network_identity::token(&context.auth_token))
                    .send()
                    .await;
            }
        }
    })
}

#[derive(serde::Deserialize)]
struct CommandsEnvelope {
    success: bool,
    commands: Option<Vec<CommandItem>>,
}

#[derive(serde::Deserialize)]
struct CommandItem {
    expires_at: Option<String>,
    id: String,
    command: String,
    payload: Option<serde_json::Value>,
}
