use std::sync::Arc;
use tokio::time::Duration;
use crate::config_manager::{ConfigManager, AgentConfig};
use crate::tasks::TaskContext;

pub fn spawn_remote_policy_consumer(
    context: Arc<TaskContext>,
    config_manager: Arc<ConfigManager>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
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
                .get(format!("{}/api/agent/policy", server_url))
                .query(&[("device_id", &context.device_id)])
                .header("x-agent-token", &context.auth_token);

            let response = match request.send().await {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let envelope: PolicyEnvelope = match response.json().await {
                Ok(e) => e,
                Err(_) => continue,
            };

            if envelope.success {
                if let Some(policy) = envelope.policy {
                    config_manager.apply_policy(policy).await;
                    tracing::info!("[RemotePolicy] Policy updated from server (v{})", config_manager.version());
                }
            }
        }
    });
}

#[derive(serde::Deserialize)]
struct PolicyEnvelope {
    success: bool,
    policy: Option<AgentConfig>,
}
