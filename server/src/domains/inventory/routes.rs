use axum::{Router, extract::{State, Path, Query}, routing::get, Json, response::IntoResponse};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;
use crate::domains::shared::DateLimitQuery;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps", get(list_apps))
        .route("/apps/:device_id", get(list_apps_for_device))
        .route("/running_apps/:device_id", get(list_running_apps))
}

async fn list_apps(
    State(state): State<Arc<AppState>>,
    Query(_query): Query<DateLimitQuery>,
) -> impl IntoResponse {
    match state.db.get_inventory(None).await {
        Ok(items) => {
            let apps_json: Vec<serde_json::Value> = items.into_iter().map(|it| {
                json!({
                    "id": it.id,
                    "device_id": it.device_id,
                    "app_name": it.app_name,
                    "version": if it.version.is_empty() { serde_json::Value::Null } else { json!(it.version) },
                    "exe_hash": it.exe_hash,
                    "verified": false,
                    "last_detected": it.timestamp.to_rfc3339()
                })
            }).collect();
            Json(json!({ "apps": apps_json }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch inventory: {}", e);
            Json(json!({ "apps": [] }))
        }
    }
}

async fn list_apps_for_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_inventory(Some(device_id.as_str())).await {
        Ok(items) => {
            let apps_json: Vec<serde_json::Value> = items.into_iter().map(|it| {
                json!({
                    "id": it.id,
                    "device_id": it.device_id,
                    "app_name": it.app_name,
                    "version": if it.version.is_empty() { serde_json::Value::Null } else { json!(it.version) },
                    "exe_hash": it.exe_hash,
                    "verified": false,
                    "last_detected": it.timestamp.to_rfc3339()
                })
            }).collect();
            Json(json!({ "apps": apps_json }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch inventory for device: {}", e);
            Json(json!({ "apps": [] }))
        }
    }
}

async fn list_running_apps(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.db.get_running_apps(device_id).await {
        Ok(apps) => {
            let apps_json: Vec<serde_json::Value> = apps.into_iter().map(|a| {
                json!({
                    "id": a.id.to_string(),
                    "device_id": a.device_id.to_string(),
                    "app_name": a.app_name,
                    "primary_title": a.primary_title,
                    "window_count": a.window_count,
                    "exe_path": a.exe_path,
                    "exe_hash": a.exe_hash,
                    "updated_at": a.updated_at.to_rfc3339()
                })
            }).collect();
            Json(json!({ "apps": apps_json }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch running apps: {}", e);
            Json(json!({ "apps": [] }))
        }
    }
}

