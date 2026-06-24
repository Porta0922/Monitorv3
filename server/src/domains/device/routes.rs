use axum::{Router, extract::{State, Path, Query}, routing::{get, post, patch}, Json, response::IntoResponse};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;
use crate::domains::shared::TzQuery;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_devices))
        .route("/:device_id", get(get_device).patch(update_device))
        .route("/register", post(register_device))
}

async fn list_devices(
    State(state): State<Arc<AppState>>,
    Query(_query): Query<TzQuery>,
) -> impl IntoResponse {
    match state.db.get_devices().await {
        Ok(devices) => Json(json!({ "devices": devices.into_iter().map(|d| crate::domains::shared::serialize_device(d, &state.config)).collect::<Vec<_>>() })),
        Err(e) => {
            tracing::error!("Failed to fetch devices: {}", e);
            Json(json!({ "devices": [] }))
        }
    }
}

async fn get_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.db.get_devices().await {
        Ok(devices) => {
            let found = devices.into_iter().find(|d| d.device_id == device_id);
            if let Some(dev) = found {
                let json_dev = crate::domains::shared::serialize_device(dev, &state.config);
                return (axum::http::StatusCode::OK, Json(json_dev));
            }
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
}

async fn update_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Json(body): Json<UpdateDeviceBody>,
) -> impl IntoResponse {
    if let Err(e) = state.db.update_device_nickname(device_id, body.nickname).await {
        tracing::error!("Failed to update device nickname: {}", e);
        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false })));
    }
    (axum::http::StatusCode::OK, Json(json!({ "success": true })))
}

#[derive(serde::Deserialize)]
struct RegisterDeviceBody {
    device_id: String,
    hostname: String,
    mac_address: Option<String>,
}

async fn register_device(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterDeviceBody>,
) -> impl IntoResponse {
    match state.db.register_device(body.hostname, body.device_id.clone(), body.mac_address, None).await {
        Ok(dev) => (axum::http::StatusCode::OK, Json(json!(crate::domains::shared::serialize_device(dev, &state.config)))),
        Err(e) => {
            tracing::error!("Failed to register device: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "failed to register device" })))
        }
    }
}

