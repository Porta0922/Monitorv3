use axum::{extract::{Query, State, Path}, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;
use crate::api::AppState;
use crate::domains::shared::{parse_iso_date, DateLimitQuery};
use uuid::Uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_usb_events))
        .route("/:device_id", get(list_device_usb_events))
}

async fn list_usb_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DateLimitQuery>,
) -> impl IntoResponse {
    let date = parse_iso_date(query.date.as_deref());
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);

    match state.db.get_usb_events(None, query.limit, date, tz_offset).await {
        Ok(events) => {
            let events_json: Vec<serde_json::Value> = events
                .into_iter()
                .map(|event| {
                    json!({
                        "timestamp": event.timestamp.to_rfc3339(),
                        "device_id": event.device_id,
                        "action": event.action,
                        "hardware_id": event.hardware_id,
                        "device_name": event.device_name,
                        "serial_number": event.serial_number,
                        "volume_label": event.volume_label,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "events": events_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch usb events: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch usb events",
                "events": []
            }))
        }
    }
}

async fn list_device_usb_events(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(query): Query<DateLimitQuery>,
) -> impl IntoResponse {
    let date = parse_iso_date(query.date.as_deref());
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);

    match state.db.get_usb_events(Some(device_id), query.limit, date, tz_offset).await {
        Ok(events) => {
            let events_json: Vec<serde_json::Value> = events
                .into_iter()
                .map(|event| {
                    json!({
                        "timestamp": event.timestamp.to_rfc3339(),
                        "action": event.action,
                        "hardware_id": event.hardware_id,
                        "device_name": event.device_name,
                        "serial_number": event.serial_number,
                        "volume_label": event.volume_label,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "events": events_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch usb events for device: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch usb events for device",
                "events": []
            }))
        }
    }
}

