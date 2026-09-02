use axum::{
    Router,
    extract::{Query, Path, State},
    Json,
    http::StatusCode,
    http::HeaderMap,
    response::IntoResponse,
};
use serde_json::to_value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;

fn verify_agent_token(headers: &axum::http::HeaderMap) -> bool {
    let expected = std::env::var("AGENT_AUTH_TOKEN").unwrap_or_else(|_| "dev-agent-token".to_string());
    headers
        .get("x-agent-token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == expected)
        .unwrap_or(false)
}

fn verify_admin_token(state: &AppState, headers: &HeaderMap) -> bool {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    state.auth.verify_token(token).is_ok()
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/osquery-policy", axum::routing::get(osquery_policy))
        .route("/policy", axum::routing::get(agent_policy))
        .route("/commands", axum::routing::get(pending_commands).post(create_command))
        .route("/commands/{id}/ack", axum::routing::post(ack_command))
}

#[derive(Deserialize)]
struct DeviceQuery {
    device_id: String,
}

#[derive(Serialize)]
struct PolicyEnvelope<T: Serialize> {
    success: bool,
    policy: Option<T>,
}

#[derive(Serialize)]
struct OsqueryPolicy {
    enabled: bool,
    tick_seconds: u64,
    min_tick_seconds: Option<u64>,
    max_tick_seconds: Option<u64>,
    profile: Option<String>,
}

async fn osquery_policy(
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !verify_agent_token(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "error": "unauthorized" })));
    }

    let profile = std::env::var("AGENT_OSQUERY_POLICY_PROFILE").ok();
    let tick_seconds: u64 = std::env::var("AGENT_OSQUERY_POLICY_TICK_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let enabled = profile.as_deref() != Some("off");

    let response = PolicyEnvelope {
        success: true,
        policy: Some(OsqueryPolicy {
            enabled,
            tick_seconds,
            min_tick_seconds: Some(30),
            max_tick_seconds: Some(900),
            profile,
        }),
    };

    (StatusCode::OK, Json(to_value(&response).unwrap()))
}

#[derive(Serialize)]
struct AgentConfigPolicy {
    window_activity_interval_secs: u64,
    heartbeat_interval_secs: u64,
    usb_detector_interval_secs: u64,
    usb_copy_interval_secs: u64,
    wifi_interval_secs: u64,
    running_apps_interval_secs: u64,
    inventory_interval_days: u64,
    heatmap_interval_secs: u64,
    resource_logger_interval_secs: u64,
    osquery_scheduler_seconds: u64,
    idle_threshold_seconds: u64,
    activity_heartbeat_seconds: u64,
    enabled_monitors: Vec<String>,
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

async fn agent_policy(
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !verify_agent_token(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "error": "unauthorized" })));
    }

    let policy = AgentConfigPolicy {
        window_activity_interval_secs: env_u64("AGENT_CONFIG_WINDOW_ACTIVITY_INTERVAL", 0),
        heartbeat_interval_secs: env_u64("AGENT_CONFIG_HEARTBEAT_INTERVAL", 0),
        usb_detector_interval_secs: env_u64("AGENT_CONFIG_USB_DETECTOR_INTERVAL", 0),
        usb_copy_interval_secs: env_u64("AGENT_CONFIG_USB_COPY_INTERVAL", 0),
        wifi_interval_secs: env_u64("AGENT_CONFIG_WIFI_INTERVAL", 0),
        running_apps_interval_secs: env_u64("AGENT_CONFIG_RUNNING_APPS_INTERVAL", 0),
        inventory_interval_days: env_u64("AGENT_CONFIG_INVENTORY_INTERVAL_DAYS", 0),
        heatmap_interval_secs: env_u64("AGENT_CONFIG_HEATMAP_INTERVAL", 0),
        resource_logger_interval_secs: env_u64("AGENT_CONFIG_RESOURCE_LOGGER_INTERVAL", 0),
        osquery_scheduler_seconds: env_u64("AGENT_CONFIG_OSQUERY_SCHEDULER_SECONDS", 0),
        idle_threshold_seconds: env_u64("AGENT_CONFIG_IDLE_THRESHOLD", 0),
        activity_heartbeat_seconds: env_u64("AGENT_CONFIG_ACTIVITY_HEARTBEAT", 0),
        enabled_monitors: Vec::new(),
    };

    let response = PolicyEnvelope {
        success: true,
        policy: Some(policy),
    };
    (StatusCode::OK, Json(to_value(&response).unwrap()))
}

#[derive(Serialize)]
struct CommandsResponse {
    success: bool,
    commands: Vec<serde_json::Value>,
}

async fn pending_commands(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceQuery>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !verify_agent_token(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "commands": [] })));
    }

    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "commands": [] }))),
    };

    let commands = match state.db.get_pending_agent_commands(device_uuid).await {
        Ok(cmds) => cmds,
        Err(e) => {
            tracing::error!("Failed to fetch pending commands: {}", e);
            Vec::new()
        }
    };

    let mut items: Vec<serde_json::Value> = Vec::new();
    for c in commands {
        let id = c.id.to_string();
        if state.db.mark_agent_command_delivered(c.id).await.is_ok() {
            items.push(serde_json::json!({
                "id": id,
                "command": c.command,
                "payload": c.payload,
            }));
        }
    }

    (StatusCode::OK, Json(to_value(&CommandsResponse { success: true, commands: items }).unwrap()))
}

#[derive(Deserialize)]
struct CreateCommandBody {
    device_id: String,
    command: String,
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

async fn create_command(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateCommandBody>,
) -> impl IntoResponse {
    if !verify_admin_token(&state, &headers) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "error": "unauthorized" })));
    }

    let device_uuid = match Uuid::parse_str(&body.device_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": "invalid device_id" }))),
    };

    let payload = body.payload.unwrap_or_else(|| serde_json::json!({}));

    match state.db.insert_agent_command(device_uuid, &body.command, payload).await {
        Ok(command_id) => (StatusCode::CREATED, Json(serde_json::json!({ "success": true, "command_id": command_id, "command": body.command }))),
        Err(e) => {
            tracing::error!("Failed to insert agent command: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": "failed to queue command" })))
        }
    }
}

async fn ack_command(
    State(state): State<Arc<AppState>>,
    Path(command_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !verify_agent_token(&headers) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false })));
    }

    let command_uuid = match Uuid::parse_str(&command_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": "invalid command id" }))),
    };

    let ack_status = body.get("status").and_then(|v| v.as_str()).unwrap_or("failed");
    let result = body.get("result").cloned().unwrap_or_else(|| serde_json::json!(null));

    match state.db.ack_agent_command(command_uuid, ack_status, result).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "success": true }))),
        Err(e) => {
            tracing::error!("Failed to ack agent command: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false })))
        }
    }
}
