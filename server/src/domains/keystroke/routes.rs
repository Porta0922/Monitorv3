use axum::{Router, extract::{State, Path, Query}, routing::get, Json, response::IntoResponse};
use serde_json::json;
use std::sync::Arc;
use crate::api::AppState;

#[derive(Debug, serde::Deserialize)]
struct HeatmapQuery {
    date: Option<String>,
    limit: Option<i64>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_heatmaps))
        .route("/:device_id", get(list_device_heatmaps))
}

async fn list_heatmaps(
    State(_state): State<Arc<AppState>>,
    Query(_q): Query<HeatmapQuery>,
) -> impl IntoResponse {
    // Placeholder: return empty list to avoid 404s until heatmap storage/query is implemented
    Json(json!({ "heatmaps": [] }))
}

async fn list_device_heatmaps(
    State(_state): State<Arc<AppState>>,
    Path(_device_id): Path<String>,
    Query(_q): Query<HeatmapQuery>,
) -> impl IntoResponse {
    Json(json!({ "heatmaps": [] }))
}

