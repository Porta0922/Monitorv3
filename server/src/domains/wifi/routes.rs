use axum::{
use crate::api::DateLimitQuery;
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use crate::api::{AppState, DateLimitQuery, parse_iso_date};
use uuid::Uuid;
use axum::extract::Path;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_wifi_events))
        .route("/:device_id", get(list_device_wifi_events))
}

async fn list_wifi_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateLimitQuery>,
) -> impl IntoResponse {
    let date = parse_iso_date(query.date.as_deref());
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    
    // We call the domain repository instead of state.db directly
    match super::repository::get_wifi_events(&state.db.pool, None, query.limit, date, tz_offset).await {
        Ok(events) => {
            let events_json: Vec<serde_json::Value> = events
                .into_iter()
                .map(|event| {
                    json!({
                        "timestamp": event.timestamp.to_rfc3339(),
                        "device_id": event.device_id,
                        "interface_name": event.interface_name,
                        "state": event.state,
                        "ssid": event.ssid,
                        "bssid": event.bssid,
                        "signal_percent": event.signal_percent,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "events": events_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch wifi events: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch wifi events",
                "events": []
            }))
        }
    }
}

async fn list_device_wifi_events(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(query): Query<DateLimitQuery>,
) -> impl IntoResponse {
    let date = parse_iso_date(query.date.as_deref());
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    
    match super::repository::get_wifi_events(&state.db.pool, Some(device_id), query.limit, date, tz_offset).await {
        Ok(events) => {
            let events_json: Vec<serde_json::Value> = events
                .into_iter()
                .map(|event| {
                    json!({
                        "timestamp": event.timestamp.to_rfc3339(),
                        "interface_name": event.interface_name,
                        "state": event.state,
                        "ssid": event.ssid,
                        "bssid": event.bssid,
                        "signal_percent": event.signal_percent,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "events": events_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch wifi events for device: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch wifi events for device",
                "events": []
            }))
        }
    }
}
