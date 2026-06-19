use std::sync::Arc;
use tokio::time::Duration;
use crate::osquery_runner::OsqueryRunner;
use super::{TaskContext, skip_interval};

fn local_osquery_scheduler_seconds() -> u64 {
    std::env::var("AGENT_OSQUERY_SCHEDULER_SECONDS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| {
            std::env::var("AGENT_OSQUERY_INTERVAL_SECONDS")
                .ok()
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .unwrap_or(0)
}

#[derive(serde::Deserialize)]
struct ServerOsqueryPolicyEnvelope {
    success: bool,
    policy: Option<ServerOsqueryPolicy>,
}

#[derive(serde::Deserialize)]
struct ServerOsqueryPolicy {
    enabled: bool,
    tick_seconds: u64,
    min_tick_seconds: Option<u64>,
    max_tick_seconds: Option<u64>,
    profile: Option<String>,
}

async fn resolve_osquery_scheduler_seconds(device_id: &str, auth_token: &str) -> u64 {
    let local_fallback = local_osquery_scheduler_seconds();

    let Some(raw_server_url) = std::env::var("AGENT_SERVER_URL").ok() else {
        return local_fallback;
    };

    let server_url = raw_server_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build();

    let Ok(client) = client else {
        return local_fallback;
    };

    let request = client
        .get(format!("{}/api/agent/osquery-policy", server_url))
        .query(&[("device_id", device_id)])
        .header("x-agent-token", auth_token);

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::debug!("Failed to fetch remote osquery policy: {}", e);
            return local_fallback;
        }
    };

    if !response.status().is_success() {
        tracing::warn!(
            "Remote osquery policy returned status {}. Falling back to local scheduler",
            response.status()
        );
        return local_fallback;
    }

    let payload: ServerOsqueryPolicyEnvelope = match response.json().await {
        Ok(body) => body,
        Err(e) => {
            tracing::warn!("Invalid remote osquery policy payload: {}", e);
            return local_fallback;
        }
    };

    if !payload.success {
        return local_fallback;
    }

    let Some(policy) = payload.policy else {
        return local_fallback;
    };

    if !policy.enabled {
        tracing::info!("osquery scheduler disabled by server policy");
        return 0;
    }

    let min_tick = policy.min_tick_seconds.unwrap_or(30).max(30);
    let max_tick = policy.max_tick_seconds.unwrap_or(900).max(min_tick);
    let effective = policy.tick_seconds.max(min_tick).min(max_tick);

    tracing::info!(
        "osquery scheduler controlled by server policy: profile={:?}, tick={}s",
        policy.profile,
        effective
    );

    effective
}

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let context_clone = context.clone();
    tokio::spawn(async move {
        let osquery_scheduler_seconds = resolve_osquery_scheduler_seconds(&context_clone.device_id, &context_clone.auth_token).await;

        if osquery_scheduler_seconds == 0 {
            tracing::info!("ℹ️ osquery security scan disabled (set server policy or AGENT_OSQUERY_SCHEDULER_SECONDS>0 to enable)");
            std::future::pending::<()>().await;
        }

        let mut runner = OsqueryRunner::new();
        let mut scan_interval = skip_interval(Duration::from_secs(osquery_scheduler_seconds.max(60)));
        loop {
            scan_interval.tick().await;

            if context_clone.keystroke_tracker.is_idle().await {
                let findings = runner.scan_due().await;
                for finding in findings {
                    let security_payload = context_clone.build_event_envelope(
                        "security",
                        1,
                        serde_json::json!({
                            "query_name":        finding.query_name,
                            "query_pack":        finding.query_pack,
                            "mitre_technique":   finding.mitre_technique,
                            "severity":          finding.severity,
                            "raw_data":          finding.raw_data,
                            "event_fingerprint": finding.event_fingerprint,
                        }),
                    );
                    context_clone.publish_or_cache("security", security_payload).await;
                }
            } else {
                tracing::debug!("User is active, skipping osquery security scan for this cycle.");
            }
        }
    })
}
