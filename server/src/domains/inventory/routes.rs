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
        Ok(items) => Json(json!({ "apps": items })),
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
        Ok(items) => Json(json!({ "apps": items })),
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
        Ok(apps) => Json(json!({ "apps": apps })),
        Err(e) => {
            tracing::error!("Failed to fetch running apps: {}", e);
            Json(json!({ "apps": [] }))
        }
    }
}

