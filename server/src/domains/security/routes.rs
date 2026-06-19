use axum::{Router, extract::{State, Path, Query, Json}, routing::{get, patch, post}, response::IntoResponse};
use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::api::AppState;

#[derive(Debug, serde::Deserialize)]
struct SecurityQuery {
    device_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    hours: Option<i64>,
    severity: Option<String>,
    mitre_technique: Option<String>,
    limit: Option<i64>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_security_events))
        .route("/:device_id", get(list_security_events_for_device))
        .route("/summary", get(security_summary))
        .route("/alerts", get(list_security_alerts).post(create_security_alert))
        .route("/alerts/:id/resolve", patch(resolve_alert))
}

async fn parse_opt_datetime(s: &Option<String>) -> Option<DateTime<Utc>> {
    s.as_ref().and_then(|v| DateTime::parse_from_rfc3339(v).ok()).map(|d| d.with_timezone(&Utc))
}

async fn list_security_events(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SecurityQuery>,
) -> impl IntoResponse {
    let device_id = q.device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let from = parse_opt_datetime(&q.from).await;
    let to = parse_opt_datetime(&q.to).await;
    let severity = q.severity.as_deref();
    let mitre = q.mitre_technique.as_deref();
    let limit = q.limit.unwrap_or(100);

    match state.db.get_security_events(device_id, from, to, severity, mitre, limit).await {
        Ok(events) => Json(json!({ "events": events })),
        Err(e) => {
            tracing::error!("Failed to fetch security events: {}", e);
            Json(json!({ "events": [] }))
        }
    }
}

async fn list_security_events_for_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(q): Query<SecurityQuery>,
) -> impl IntoResponse {
    let from = parse_opt_datetime(&q.from).await;
    let to = parse_opt_datetime(&q.to).await;
    let severity = q.severity.as_deref();
    let mitre = q.mitre_technique.as_deref();
    let limit = q.limit.unwrap_or(100);

    match state.db.get_security_events(Some(device_id), from, to, severity, mitre, limit).await {
        Ok(events) => Json(json!({ "events": events })),
        Err(e) => {
            tracing::error!("Failed to fetch security events for device: {}", e);
            Json(json!({ "events": [] }))
        }
    }
}

async fn security_summary(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SecurityQuery>,
) -> impl IntoResponse {
    let device_id = q.device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let from = parse_opt_datetime(&q.from).await;
    let to = parse_opt_datetime(&q.to).await;

    match state.db.get_security_summary(device_id, from, to).await {
        Ok(summary) => Json(json!(summary)),
        Err(e) => {
            tracing::error!("Failed to fetch security summary: {}", e);
            Json(json!({}))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct AlertsQuery {
    device_id: Option<String>,
    severity: Option<String>,
    resolved: Option<bool>,
    limit: Option<i64>,
}

async fn list_security_alerts(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AlertsQuery>,
) -> impl IntoResponse {
    let device_id = q.device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let severity = q.severity.as_deref();
    let resolved = q.resolved;
    let limit = q.limit.unwrap_or(100);

    match state.db.get_security_alerts(device_id, severity, resolved, limit).await {
        Ok(alerts) => Json(json!({ "alerts": alerts })),
        Err(e) => {
            tracing::error!("Failed to fetch security alerts: {}", e);
            Json(json!({ "alerts": [] }))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct ResolvePayload {
    resolution_notes: Option<String>,
}

async fn resolve_alert(
    State(state): State<Arc<AppState>>,
    Path(alert_id): Path<i64>,
    Json(payload): Json<ResolvePayload>,
) -> impl IntoResponse {
    match state.db.resolve_security_alert(alert_id, payload.resolution_notes.as_deref()).await {
        Ok(Some(alert)) => (StatusCode::OK, Json(serde_json::to_value(json!({ "success": true, "alert": alert })).unwrap())),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false }))),
        Err(e) => {
            tracing::error!("Failed to resolve alert: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false })))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct CreateAlertBody {
    device_id: String,
    alert_type: String,
    app_name: Option<String>,
    exe_hash: Option<String>,
    description: String,
    severity: String,
}

async fn create_security_alert(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateAlertBody>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&body.device_id) {
        Ok(u) => u,
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": "invalid device_id" }))),
    };

    match state.db.insert_security_alert(device_uuid, &body.alert_type, body.app_name.as_deref(), body.exe_hash.as_deref(), &body.description, &body.severity).await {
        Ok(alert) => (StatusCode::OK, Json(serde_json::to_value(json!({ "success": true, "alert": alert })).unwrap())),
        Err(e) => {
            tracing::error!("Failed to create security alert: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false })))
        }
    }
}

