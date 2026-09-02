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
        "flush_cache" => {
            let count = context.cache.get_unsynced_event_count().await.unwrap_or(0);
            let _ = context.cache.clear_unsynced_events().await;
            serde_json::json!({
                "status": "ok",
                "cleared_events": count,
            })
        }
        "reconnect" => {
            let rabbitmq_url = std::env::var("RABBITMQ_URL")
                .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/%2F".to_string());
            let connect_result = crate::rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url).await;
            match connect_result {
                Ok(new_publisher) => {
                    let mut pub_state = context.publisher.write().await;
                    *pub_state = Some(Arc::new(new_publisher));
                    serde_json::json!({ "status": "ok", "message": "Reconnected to RabbitMQ" })
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    serde_json::json!({ "status": "error", "message": err_msg })
                }
            }
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
        "force_update" => {
            match crate::updater::apply_update().await {
                crate::updater::ApplyUpdateResult::Updated { version } => {
                    serde_json::json!({ "status": "ok", "message": format!("Update v{} applied", version) })
                }
                crate::updater::ApplyUpdateResult::UpToDate => {
                    serde_json::json!({ "status": "ok", "message": "already up to date" })
                }
                crate::updater::ApplyUpdateResult::Failed(e) => {
                    serde_json::json!({ "status": "error", "message": e })
                }
            }
        }
        "uninstall" => {
            match crate::uninstall::spawn_uninstall() {
                Ok(_) => serde_json::json!({ "status": "ok", "message": "Uninstall script launched (self-elevating). Agent will be removed shortly." }),
                Err(e) => serde_json::json!({ "status": "error", "message": format!("Failed to launch uninstall: {}", e) }),
            }
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
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let server_url = match std::env::var("AGENT_SERVER_URL") {
                Ok(url) => url.trim_end_matches('/').to_string(),
                Err(_) => continue,
            };

            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
            {
                Ok(c) => c,
                Err(_) => continue,
            };

            let request = client
                .get(format!("{}/api/agent/commands", server_url))
                .query(&[("device_id", &context.device_id)])
                .header("x-agent-token", &context.auth_token);

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
                tracing::info!("[CommandChannel] Received command: {} (id={})", cmd.command, cmd.id);
                let result = process_command(
                    &cmd.command,
                    cmd.payload,
                    &context,
                    &supervisor,
                    &config_manager,
                )
                .await;

                let ack_client = match reqwest::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                {
                    Ok(c) => c,
                    Err(_) => break,
                };

                let ack_url = format!("{}/api/agent/commands/{}/ack", server_url, cmd.id);
                let _ = ack_client
                    .post(&ack_url)
                    .json(&serde_json::json!({
                        "device_id": context.device_id,
                        "status": "ok",
                        "result": result,
                    }))
                    .header("x-agent-token", &context.auth_token)
                    .send()
                    .await;
            }
        }
    });
}

#[derive(serde::Deserialize)]
struct CommandsEnvelope {
    success: bool,
    commands: Option<Vec<CommandItem>>,
}

#[derive(serde::Deserialize)]
struct CommandItem {
    id: String,
    command: String,
    payload: Option<serde_json::Value>,
}
