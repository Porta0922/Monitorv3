use axum::{Router, extract::{State, Path, Query}, routing::{get, post}, Json, response::IntoResponse};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::api::AppState;
use crate::domains::shared::{ActivityLogFilters, parse_time_bounds};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_logs))
        .route("/:device_id", get(list_device_logs))
        .route("/ingest", post(ingest_logs))
}

async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<ActivityLogFilters>,
) -> impl IntoResponse {
    let (from, to) = parse_time_bounds(&filters);
    match state.db.get_activity_logs_filtered(None, from, to, filters.limit).await {
        Ok(logs) => Json(json!({ "logs": logs.into_iter().map(|l| json!({ "id": l.id, "device_id": l.device_id, "app_name": l.app_name, "window_title": l.window_title, "duration_seconds": l.duration_seconds, "timestamp": l.timestamp.to_rfc3339() })).collect::<Vec<_>>() })),
        Err(e) => {
            tracing::error!("Failed to fetch activity logs: {}", e);
            Json(json!({ "logs": [] }))
        }
    }
}

async fn list_device_logs(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(filters): Query<ActivityLogFilters>,
) -> impl IntoResponse {
    let (from, to) = parse_time_bounds(&filters);
    match state.db.get_activity_logs_filtered(Some(device_id), from, to, filters.limit).await {
        Ok(logs) => Json(json!({ "logs": logs.into_iter().map(|l| json!({ "id": l.id, "device_id": l.device_id, "app_name": l.app_name, "window_title": l.window_title, "duration_seconds": l.duration_seconds, "timestamp": l.timestamp.to_rfc3339() })).collect::<Vec<_>>() })),
        Err(e) => {
            tracing::error!("Failed to fetch activity logs for device: {}", e);
            Json(json!({ "logs": [] }))
        }
    }
}

#[derive(serde::Deserialize)]
struct IngestPayload {
    device_id: String,
    events: Vec<IngestEvent>,
}

#[derive(serde::Deserialize)]
struct IngestEvent {
    app_name: String,
    window_title: String,
    duration_seconds: i64,
}

async fn ingest_logs(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IngestPayload>,
) -> impl IntoResponse {
    for ev in body.events.iter() {
        if let Err(e) = state.db.insert_activity_log(body.device_id.clone(), ev.app_name.clone(), ev.window_title.clone(), ev.duration_seconds).await {
            tracing::error!("Failed to insert activity log: {}", e);
        }
    }
    (axum::http::StatusCode::OK, Json(json!({ "success": true })))
}

