use axum::{Router, extract::{State, Path, Query}, routing::{get, post}, Json, response::IntoResponse};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;
use crate::domains::shared::TzQuery;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_devices))
        .route("/search",get(super::search::search))
        .route("/:device_id/sessions",get(super::sessions::list))
        .route("/:device_id/sessions/export",get(super::sessions::export))
        .route("/:device_id", get(get_device).patch(update_device))
        .route("/register", post(register_device))
}

async fn list_devices(
    State(state): State<Arc<AppState>>,
    Query(_query): Query<TzQuery>,
) -> impl IntoResponse {
    match state.db.get_devices().await {
        Ok(devices) => {
            let ids: Vec<_> = devices.iter().map(|d| d.device_id).collect();
            let health = sqlx::query_scalar::<_, serde_json::Value>(r#"
                SELECT COALESCE(jsonb_object_agg(device_id::text,jsonb_build_object(
                    'status',payload->'status','monitoring_paused',payload->'monitoring_paused',
                    'observed_at',observed_at,'stale',observed_at < now()-interval '900 seconds')), '{}'::jsonb)
                FROM device_telemetry_state WHERE kind='health' AND device_id=ANY($1)
            "#).bind(ids).fetch_one(&state.db.pool).await;
            if let Err(ref error) = health { tracing::warn!(%error, "Fleet health unavailable"); }
            let values: Vec<_> = devices.into_iter().map(|device| {
                let id = device.device_id.to_string();
                let mut value = crate::domains::shared::serialize_device(device, &state.config);
                value["agent_health"] = health.as_ref().ok().and_then(|h|h.get(&id)).cloned().unwrap_or(serde_json::Value::Null);
                value["agent_health_error"] = json!(health.is_err());
                value
            }).collect();
            Json(json!({"devices":values})).into_response()
        },
        Err(e) => {
            tracing::error!("Failed to fetch devices: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Devices unavailable"}))).into_response()
        }
    }
}

async fn get_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.db.get_device_by_id(device_id).await {
        Ok(Some(dev)) => {
            let json_dev = crate::domains::shared::serialize_device(dev, &state.config);
            (axum::http::StatusCode::OK, Json(json_dev))
        }
        Ok(None) => {
            (axum::http::StatusCode::NOT_FOUND, Json(json!({ "error": "device not found" })))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "failed to fetch device" })))
        }
    }
}

#[derive(serde::Deserialize)]
struct UpdateDeviceBody {
    nickname: Option<String>,
    group_name: Option<String>,
    desired_version: Option<String>,
}

async fn update_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Json(body): Json<UpdateDeviceBody>,
) -> impl IntoResponse {
    if body.nickname.as_ref().is_some_and(|v|v.len()>100) || body.group_name.as_ref().is_some_and(|v|v.len()>100) || body.desired_version.as_ref().is_some_and(|v|v.len()>64) {
        return (axum::http::StatusCode::BAD_REQUEST,Json(json!({"error":"Metadata too long"})));
    }
    match sqlx::query("UPDATE devices SET nickname=$2,group_name=COALESCE($3,group_name),desired_version=CASE WHEN $4::text IS NULL THEN desired_version ELSE NULLIF($4,'') END,updated_at=now() WHERE device_id=$1")
        .bind(device_id).bind(body.nickname).bind(body.group_name.map(|v|v.trim().to_owned())).bind(body.desired_version.map(|v|v.trim().to_owned())).execute(&state.db.pool).await {
        Ok(result) if result.rows_affected()>0=>(axum::http::StatusCode::OK,Json(json!({"success":true}))),
        Ok(_)=>(axum::http::StatusCode::NOT_FOUND,Json(json!({"error":"Device not found"}))),
        Err(error)=>{tracing::error!(%error,"Equipment update failed");(axum::http::StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"Update failed"})))}
    }
}

#[derive(serde::Deserialize)]
struct RegisterDeviceBody {
    device_id: String,
    hostname: String,
    mac_address: Option<String>,
    version: Option<String>,
}

async fn register_device(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterDeviceBody>,
) -> impl IntoResponse {
    match state.db.register_device(body.hostname, body.device_id.clone(), body.mac_address, None, body.version).await {
        Ok(dev) => (axum::http::StatusCode::OK, Json(json!(crate::domains::shared::serialize_device(dev, &state.config)))),
        Err(e) => {
            tracing::error!("Failed to register device: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "failed to register device" })))
        }
    }
}

