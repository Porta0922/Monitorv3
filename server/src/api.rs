// REST API routes using Axum
use axum::{
    Router,
    routing::{get, post, patch},
    Json,
    extract::{Path, Query, State},
    middleware::Next,
    http::{Method, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{Duration, Utc};
use tower_http::cors::{CorsLayer, AllowOrigin, AllowHeaders};
use futures::stream::{self, StreamExt};
use std::convert::Infallible;
use tokio::time::timeout;
use lapin::{Connection, ConnectionProperties};

use crate::auth::AuthManager;
use crate::config::RuntimeConfig;
use crate::postgres_db::Database;

pub struct AppState {
    pub auth: AuthManager,
    pub db: Database,
    pub config: RuntimeConfig,
    pub rabbitmq_url: String,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    // Configure CORS layer
    let cors = CorsLayer::permissive()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin
                .as_bytes()
                .eq(b"http://localhost:5173")
                || origin.as_bytes().eq(b"http://localhost:3000")
                || origin.as_bytes().eq(b"http://127.0.0.1:5173")
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS, Method::PATCH])
        .allow_headers(AllowHeaders::any());

    // Create the main router with protected endpoints
    let protected_router = Router::new()
        // Protected endpoints (require JWT)
        .route("/devices", get(list_devices))
        .route("/devices/:device_id", get(get_device))
        .route("/devices/:device_id", patch(update_device))
        .route("/logs/ingest", post(ingest_activity_logs))
        .route("/logs", get(query_activity_logs))
        .route("/logs/:device_id", get(get_device_logs))
        .route("/inventory/apps", get(list_all_apps))
        .route("/inventory/apps/:device_id", get(list_device_apps))
        .route("/usb", get(list_usb_events))
        .route("/usb/:device_id", get(list_device_usb_events))
        .route("/wifi", get(list_wifi_events))
        .route("/wifi/:device_id", get(list_device_wifi_events))
        .route("/history", get(get_history))
        .route("/hourly", get(get_hourly))
        .route("/available_dates", get(get_available_dates))
        .route("/active_vs_idle", get(get_active_vs_idle))
        .route("/live_devices", get(get_live_devices))
        .route("/export/csv", get(export_csv))
        .route("/audit", get(list_audit_events))
        
        // NEW: Input Heatmaps
        .route("/heatmaps/upload", post(upload_heatmap))
        .route("/heatmaps/:device_id", get(get_device_heatmaps))
        .route("/heatmaps/:device_id/current", get(get_current_heatmap))
        
        // NEW: Security Alerts
        .route("/alerts", get(list_security_alerts))
        .route("/alerts/:device_id", get(list_device_alerts))
        .route("/alerts/:alert_id/resolve", patch(resolve_alert))
        .route("/alerts/process-protection", post(record_termination_attempt))
        
        // Apply JWT middleware to protected routes only
        .layer(axum::middleware::from_fn(verify_jwt_middleware))
        .with_state(state.clone());

    // Public routes
    let public_router = Router::new()
        .route("/health", get(health_check))
        .route("/readiness", get(readiness_check))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route("/devices/register", post(register_device))
        .route("/overview", get(get_overview))
        .route("/top_apps", get(get_top_apps))
        .route("/metrics/summary", get(get_metrics_summary))
        .route("/stream", get(stream_events))
        .with_state(state);

    // Combine routers with /api prefix and apply CORS
    let api_router = Router::new()
        .nest("/api", public_router.merge(protected_router));

    api_router
        .layer(cors)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "ActivityMonitor Server",
        "version": "0.1.0"
    }))
}

async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db_ready = timeout(
        std::time::Duration::from_millis(state.config.readiness_timeout_ms),
        state.db.ping(),
    )
    .await
    .map(|res| res.is_ok())
    .unwrap_or(false);

    let rabbit_ready = timeout(
        std::time::Duration::from_millis(state.config.readiness_timeout_ms),
        Connection::connect(&state.rabbitmq_url, ConnectionProperties::default()),
    )
    .await
    .map(|res| res.is_ok())
    .unwrap_or(false);

    let overall = if db_ready && rabbit_ready {
        "ok"
    } else if db_ready || rabbit_ready {
        "degraded"
    } else {
        "down"
    };

    Json(json!({
        "status": overall,
        "checks": {
            "database": if db_ready { "ok" } else { "down" },
            "rabbitmq": if rabbit_ready { "ok" } else { "down" }
        },
        "thresholds": {
            "online_threshold_seconds": state.config.online_threshold_seconds,
            "live_threshold_seconds": state.config.live_threshold_seconds,
            "stale_threshold_seconds": state.config.stale_threshold_seconds
        }
    }))
}

async fn register_user(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract username, password from payload
    // TODO: Hash password using AuthManager
    // TODO: Store in database
    
    Json(json!({
        "success": true,
        "message": "User registered successfully"
    }))
}

async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> Response {
    let username = _payload
        .get("username")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let has_password = _payload
        .get("password")
        .and_then(|p| p.as_str())
        .is_some();

    if has_password {
        // TODO: Fetch user from database and verify password hash
        let token_result = state.auth.issue_token(&username, 24);
        if let Ok(token) = token_result {
            if state.config.audit_log_enabled {
                let db = state.db.clone();
                let actor = username.clone();
                tokio::spawn(async move {
                    let _ = db
                        .insert_audit_event(&actor, "auth.login.success", "api", None)
                        .await;
                });
            }
            return Json(json!({
                "success": true,
                "token": token,
                "expires_in": 86400
            }))
            .into_response();
        }
    }

    if state.config.audit_log_enabled {
        let db = state.db.clone();
        let actor = username.clone();
        tokio::spawn(async move {
            let _ = db
                .insert_audit_event(&actor, "auth.login.failed", "api", None)
                .await;
        });
    }
    
    Json(json!({
        "success": false,
        "error": "Invalid credentials"
    })).into_response()
}

async fn register_device(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let device_id = match payload.get("device_id").and_then(|value| value.as_str()) {
        Some(value) => value.to_string(),
        None => {
            return Json(json!({
                "success": false,
                "error": "device_id is required"
            }));
        }
    };

    let hostname = payload
        .get("hostname")
        .and_then(|value| value.as_str())
        .unwrap_or(&device_id)
        .to_string();

    let mac_address = payload
        .get("mac_address")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    let nickname = payload
        .get("nickname")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    match state.db.register_device(hostname, device_id, mac_address, nickname).await {
        Ok(device) => Json(json!({
            "success": true,
            "device": serialize_device(device, &state.config)
        })),
        Err(error) => {
            tracing::error!("Failed to register device: {}", error);
            Json(json!({
                "success": false,
                "error": "Failed to register device"
            }))
        }
    }
}

async fn list_devices(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_devices().await {
        Ok(devices) => {
            let totals_map: HashMap<Uuid, (i64, i64, i64, i64, i64)> = match state.db.get_device_time_totals_today().await {
                Ok(totals) => totals
                    .into_iter()
                    .map(|item| {
                        (
                            item.device_id,
                            (
                                item.active_seconds,
                                item.idle_seconds,
                                item.keys_count,
                                item.mouse_moves_count,
                                item.clicks_count,
                            ),
                        )
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!("Failed to fetch device time totals: {}", e);
                    HashMap::new()
                }
            };

            let device_json: Vec<serde_json::Value> = devices
                .into_iter()
                .map(|device| {
                    let mut value = serialize_device(device.clone(), &state.config);
                    let (active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count) = totals_map
                        .get(&device.device_id)
                        .copied()
                        .unwrap_or((0, 0, 0, 0, 0));

                    if let Some(obj) = value.as_object_mut() {
                        obj.insert("active_time_today_seconds".to_string(), json!(active_seconds));
                        obj.insert("idle_time_today_seconds".to_string(), json!(idle_seconds));
                        obj.insert("keys_today".to_string(), json!(keys_count));
                        obj.insert("mouse_moves_today".to_string(), json!(mouse_moves_count));
                        obj.insert("mouse_clicks_today".to_string(), json!(clicks_count));
                    }

                    value
                })
                .collect();
            
            Json(json!({
                "success": true,
                "devices": device_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch devices: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch devices",
                "devices": []
            }))
        }
    }
}

async fn get_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.db.get_devices().await {
        Ok(devices) => {
            if let Some(device) = devices.into_iter().find(|device| device.device_id == device_id) {
                let (active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count) = match state.db.get_single_device_time_totals_today(device_id).await {
                    Ok(totals) => (
                        totals.active_seconds,
                        totals.idle_seconds,
                        totals.keys_count,
                        totals.mouse_moves_count,
                        totals.clicks_count,
                    ),
                    Err(_) => (0, 0, 0, 0, 0),
                };

                let mut device_json = serialize_device(device, &state.config);
                if let Some(obj) = device_json.as_object_mut() {
                    obj.insert("active_time_today_seconds".to_string(), json!(active_seconds));
                    obj.insert("idle_time_today_seconds".to_string(), json!(idle_seconds));
                    obj.insert("keys_today".to_string(), json!(keys_count));
                    obj.insert("mouse_moves_today".to_string(), json!(mouse_moves_count));
                    obj.insert("mouse_clicks_today".to_string(), json!(clicks_count));
                }

                return Json(json!({
                    "success": true,
                    "device": device_json
                }));
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch device: {}", e);
        }
    }
    
    Json(json!({
        "success": true,
        "device": null
    }))
}

async fn update_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let nickname = payload
        .get("nickname")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string());

    match state.db.update_device_nickname(device_id, nickname.clone()).await {
        Ok(_) => {
            if state.config.audit_log_enabled {
                let detail = format!("nickname={}", nickname.clone().unwrap_or_default());
                let _ = state
                    .db
                    .insert_audit_event(
                        "api/device",
                        "device.nickname.update",
                        &device_id.to_string(),
                        Some(detail.as_str()),
                    )
                    .await;
            }

            Json(json!({
                "success": true,
                "message": "Device updated successfully",
                "device_id": device_id,
                "nickname": nickname,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to update device {}: {}", device_id, e);
            Json(json!({
                "success": false,
                "error": "Failed to update device",
            }))
        }
    }
}

async fn ingest_activity_logs(
    State(state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract activity logs batch
    // TODO: Insert into activity_logs hypertable
    
    Json(json!({
        "success": true,
        "message": "Activity logs ingested"
    }))
}

async fn query_activity_logs(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<ActivityLogFilters>,
) -> impl IntoResponse {
    let (from, to) = parse_time_bounds(&filters);
    match state.db.get_activity_logs_filtered(None, from, to, filters.limit).await {
        Ok(logs) => {
            let logs_json: Vec<serde_json::Value> = logs.iter().map(|l| {
                json!({
                    "id": l.id,
                    "device_id": l.device_id,
                    "app_name": l.app_name,
                    "window_title": l.window_title,
                    "duration_seconds": l.duration_seconds,
                    "timestamp": l.timestamp.to_rfc3339(),
                })
            }).collect();
            
            Json(json!({
                "success": true,
                "logs": logs_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch activity logs: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch activity logs",
                "logs": []
            }))
        }
    }
}

async fn get_device_logs(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(filters): Query<ActivityLogFilters>,
) -> impl IntoResponse {
    let device_id_str = device_id.to_string();
    let (from, to) = parse_time_bounds(&filters);
    
    match state.db.get_activity_logs_filtered(Some(device_id), from, to, filters.limit).await {
        Ok(logs) => {
            let logs_json: Vec<serde_json::Value> = logs.iter().map(|l| {
                json!({
                    "id": l.id,
                    "device_id": l.device_id,
                    "app_name": l.app_name,
                    "window_title": l.window_title,
                    "duration_seconds": l.duration_seconds,
                    "timestamp": l.timestamp.to_rfc3339(),
                })
            }).collect();
            
            Json(json!({
                "success": true,
                "device_id": device_id_str,
                "logs": logs_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device logs: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch device logs",
                "device_id": device_id_str,
                "logs": []
            }))
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ActivityLogFilters {
    limit: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    hours: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct ListLimitQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct DeviceDateQuery {
    device_id: Option<String>,
    date: Option<String>,
    tz_offset_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
struct ActiveIdleQuery {
    days: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct LiveDevicesQuery {
    live_only: Option<bool>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct AuditQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct ExportCsvQuery {
    device_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    tz_offset_minutes: Option<i32>,
}

fn format_duration(seconds: i64) -> String {
    let safe_seconds = seconds.max(0);
    let hours = safe_seconds / 3600;
    let minutes = (safe_seconds % 3600) / 60;
    let rem_seconds = safe_seconds % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, rem_seconds)
    } else {
        format!("{}s", rem_seconds)
    }
}

fn parse_iso_date(value: Option<&str>) -> Option<chrono::NaiveDate> {
    value.and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
}

fn csv_escape(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

fn parse_time_bounds(filters: &ActivityLogFilters) -> (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>) {
    let from = filters
        .from
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));

    let to = filters
        .to
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));

    if from.is_none() {
        if let Some(hours) = filters.hours {
            if hours > 0 {
                return (Some(Utc::now() - Duration::hours(hours)), to);
            }
        }
    }

    (from, to)
}

async fn list_all_apps(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_inventory(None).await {
        Ok(apps) => {
            let app_json: Vec<serde_json::Value> = apps
                .into_iter()
                .enumerate()
                .map(|(idx, app)| {
                    json!({
                        "id": idx + 1,
                        "device_id": app.device_id,
                        "app_name": app.app_name,
                        "version": app.version,
                        "exe_hash": app.exe_hash,
                        "verified": false,
                        "last_detected": app.timestamp.to_rfc3339(),
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "apps": app_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch inventory apps: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch inventory apps",
                "apps": []
            }))
        }
    }
}

async fn list_device_apps(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    let device_id_str = device_id.to_string();
    match state.db.get_inventory(Some(&device_id_str)).await {
        Ok(apps) => {
            let app_json: Vec<serde_json::Value> = apps
                .into_iter()
                .enumerate()
                .map(|(idx, app)| {
                    json!({
                        "id": idx + 1,
                        "device_id": app.device_id,
                        "app_name": app.app_name,
                        "version": app.version,
                        "exe_hash": app.exe_hash,
                        "verified": false,
                        "last_detected": app.timestamp.to_rfc3339(),
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "device_id": device_id_str,
                "apps": app_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device inventory apps: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch device inventory apps",
                "device_id": device_id_str,
                "apps": []
            }))
        }
    }
}

async fn list_usb_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match state.db.get_usb_events(None, query.limit).await {
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
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    let device_id_str = device_id.to_string();

    match state.db.get_usb_events(Some(device_id), query.limit).await {
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
                "device_id": device_id_str,
                "events": events_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device usb events: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch device usb events",
                "device_id": device_id_str,
                "events": []
            }))
        }
    }
}

async fn list_wifi_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    match state.db.get_wifi_events(None, query.limit).await {
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
    Query(query): Query<ListLimitQuery>,
) -> impl IntoResponse {
    let device_id_str = device_id.to_string();

    match state.db.get_wifi_events(Some(device_id), query.limit).await {
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
                "device_id": device_id_str,
                "events": events_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device wifi events: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch device wifi events",
                "device_id": device_id_str,
                "events": []
            }))
        }
    }
}

async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceDateQuery>,
) -> impl IntoResponse {
    let Some(device_id_raw) = query.device_id.as_deref() else {
        return Json(json!({
            "success": false,
            "error": "device_id is required",
            "history": []
        }));
    };

    let Ok(device_id) = Uuid::parse_str(device_id_raw) else {
        return Json(json!({
            "success": false,
            "error": "invalid device_id",
            "history": []
        }));
    };

    let selected_date = parse_iso_date(query.date.as_deref()).unwrap_or_else(|| Utc::now().date_naive());
    let tz_offset_minutes = query.tz_offset_minutes.unwrap_or(0);

    match state
        .db
        .get_device_history_for_date(device_id, selected_date, tz_offset_minutes)
        .await
    {
        Ok(rows) => {
            let history: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(app_name, window_title, seconds, intervals)| {
                    let is_idle = app_name.to_lowercase().contains("idle")
                        || window_title.to_lowercase().contains("idle");
                    json!({
                        "app": app_name,
                        "title": window_title,
                        "seconds": seconds,
                        "duration": format_duration(seconds),
                        "intervals": intervals,
                        "is_idle": is_idle,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "device_id": device_id,
                "date": selected_date.to_string(),
                "tz_offset_minutes": tz_offset_minutes,
                "history": history,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch history: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch history",
                "history": []
            }))
        }
    }
}

async fn get_hourly(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceDateQuery>,
) -> impl IntoResponse {
    let Some(device_id_raw) = query.device_id.as_deref() else {
        return Json(json!({
            "success": false,
            "error": "device_id is required",
            "hourly": []
        }));
    };

    let Ok(device_id) = Uuid::parse_str(device_id_raw) else {
        return Json(json!({
            "success": false,
            "error": "invalid device_id",
            "hourly": []
        }));
    };

    let selected_date = parse_iso_date(query.date.as_deref()).unwrap_or_else(|| Utc::now().date_naive());
    let tz_offset_minutes = query.tz_offset_minutes.unwrap_or(0);

    match state
        .db
        .get_device_hourly_for_date(device_id, selected_date, tz_offset_minutes)
        .await
    {
        Ok(rows) => {
            let mut by_hour = std::collections::HashMap::new();
            for (hour, active_seconds, idle_seconds) in rows {
                by_hour.insert(hour, (active_seconds, idle_seconds));
            }

            let hourly: Vec<serde_json::Value> = (0..24)
                .map(|hour| {
                    let (active_seconds, idle_seconds) = by_hour.get(&hour).copied().unwrap_or((0, 0));
                    json!({
                        "hour": hour,
                        "label": format!("{:02}:00", hour),
                        "active_seconds": active_seconds,
                        "idle_seconds": idle_seconds,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "device_id": device_id,
                "date": selected_date.to_string(),
                "tz_offset_minutes": tz_offset_minutes,
                "hourly": hourly,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch hourly data: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch hourly data",
                "hourly": []
            }))
        }
    }
}

async fn get_available_dates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeviceDateQuery>,
) -> impl IntoResponse {
    let device_uuid = query
        .device_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok());

    let tz_offset_minutes = query.tz_offset_minutes.unwrap_or(0);

    match state.db.get_available_dates(device_uuid, 90, tz_offset_minutes).await {
        Ok(dates) => {
            let items: Vec<String> = dates.into_iter().map(|d| d.to_string()).collect();
            Json(json!({
                "success": true,
                "tz_offset_minutes": tz_offset_minutes,
                "dates": items,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch available dates: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch available dates",
                "dates": []
            }))
        }
    }
}

async fn get_active_vs_idle(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ActiveIdleQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(7).clamp(1, 90);
    let since = Utc::now() - Duration::days(days);

    match state.db.get_active_vs_idle_since(since).await {
        Ok(rows) => {
            let data: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(device_id, active_seconds, idle_seconds)| {
                    let total = active_seconds + idle_seconds;
                    let active_pct = if total > 0 {
                        (active_seconds as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };

                    json!({
                        "device_id": device_id,
                        "active_seconds": active_seconds,
                        "idle_seconds": idle_seconds,
                        "active": format_duration(active_seconds),
                        "idle": format_duration(idle_seconds),
                        "active_pct": (active_pct * 10.0).round() / 10.0,
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "days": days,
                "data": data,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch active_vs_idle: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch active_vs_idle",
                "data": []
            }))
        }
    }
}

async fn get_live_devices(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LiveDevicesQuery>,
) -> impl IntoResponse {
    match state.db.get_live_devices_activity().await {
        Ok(rows) => {
            let now = Utc::now();
            let live_threshold = state.config.live_threshold_seconds.max(1);
            let stale_threshold = state.config.stale_threshold_seconds.max(1);
            let limit = query.limit.unwrap_or(50).max(1);
            let data: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|row| {
                    let ago_sec = (now - row.timestamp).num_seconds().max(0);
                    json!({
                        "device_id": row.device_id,
                        "app": row.app_name,
                        "title": row.window_title,
                        "last_seen": row.timestamp.to_rfc3339(),
                        "ago_sec": ago_sec,
                        "is_live": ago_sec < live_threshold,
                        "is_stale": ago_sec >= stale_threshold,
                        "is_idle": row.app_name.to_lowercase().contains("idle") || row.window_title.to_lowercase().contains("idle"),
                        "duration": format_duration(row.duration_seconds),
                    })
                })
                .filter(|row| {
                    if query.live_only.unwrap_or(false) {
                        row.get("is_live")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false)
                    } else {
                        true
                    }
                })
                .take(limit)
                .collect();

            Json(json!({
                "success": true,
                "devices": data,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch live devices: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch live devices",
                "devices": []
            }))
        }
    }
}

async fn export_csv(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExportCsvQuery>,
) -> impl IntoResponse {
    let today = Utc::now().date_naive();
    let from = parse_iso_date(query.from.as_deref()).unwrap_or(today - chrono::Days::new(7));
    let to = parse_iso_date(query.to.as_deref()).unwrap_or(today);
    let device_uuid = query
        .device_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok());
    let tz_offset_minutes = query.tz_offset_minutes.unwrap_or(0);

    match state
        .db
        .get_activity_logs_for_export(device_uuid, from, to, tz_offset_minutes)
        .await
    {
        Ok(rows) => {
            let mut csv = String::from("timestamp,device_id,app_name,window_title,duration_seconds\n");
            for row in rows {
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    csv_escape(&row.timestamp.to_rfc3339()),
                    csv_escape(&row.device_id.to_string()),
                    csv_escape(&row.app_name),
                    csv_escape(&row.window_title),
                    row.duration_seconds,
                ));
            }

            if state.config.audit_log_enabled {
                let actor = "api/export";
                let target = query.device_id.as_deref().unwrap_or("all-devices");
                let details = format!(
                    "from={},to={},tz_offset_minutes={},rows={}",
                    from,
                    to,
                    tz_offset_minutes,
                    csv.lines().count().saturating_sub(1)
                );
                let _ = state
                    .db
                    .insert_audit_event(actor, "csv.export", target, Some(details.as_str()))
                    .await;
            }

            let filename = format!("activity_{}_{}.csv", from, to);
            (
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        Box::leak(format!("attachment; filename={}", filename).into_boxed_str()),
                    ),
                ],
                csv,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to export csv: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to export csv"
            }))
            .into_response()
        }
    }
}

async fn list_audit_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditQuery>,
) -> impl IntoResponse {
    match state.db.get_audit_events(query.limit.unwrap_or(100)).await {
        Ok(rows) => {
            let events: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "actor": e.actor,
                        "action": e.action,
                        "target": e.target,
                        "details": e.details,
                        "created_at": e.created_at.to_rfc3339(),
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "events": events,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch audit events: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch audit events",
                "events": [],
            }))
        }
    }
}

async fn get_metrics_summary(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state
        .db
        .get_operational_metrics(
            state.config.online_threshold_seconds,
            state.config.stale_threshold_seconds,
        )
        .await
    {
        Ok(m) => Json(json!({
            "success": true,
            "metrics": {
                "devices_total": m.devices_total,
                "devices_online": m.devices_online,
                "devices_stale": m.devices_stale,
                "activities_last_hour": m.activities_last_hour,
                "input_rows_today": m.input_rows_today,
                "newest_activity_age_seconds": m.newest_activity_age_seconds,
            },
            "thresholds": {
                "online_threshold_seconds": state.config.online_threshold_seconds,
                "stale_threshold_seconds": state.config.stale_threshold_seconds,
            }
        })),
        Err(e) => {
            tracing::error!("Failed to fetch metrics summary: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch metrics summary"
            }))
        }
    }
}

// ============================================================================
// INPUT HEATMAP ENDPOINTS (NEW)
// ============================================================================

async fn upload_heatmap(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract heatmap data from payload
    // TODO: Validate heatmap format
    // TODO: Insert into input_activity_heatmaps hypertable
    // TODO: Update input_activity_daily_summary
    
    Json(json!({
        "success": true,
        "message": "Heatmap uploaded successfully",
        "heatmap_id": "HEATMAP-001"
    }))
}

async fn get_device_heatmaps(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query input_activity_heatmaps for specific device
    // TODO: Apply time filters (last 7 days, last 30 days, etc)
    // TODO: Optionally aggregate by hour/day
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "heatmaps": []
    }))
}

async fn get_current_heatmap(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Get most recent heatmap for device
    // TODO: Return with real-time visualization data
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "current_heatmap": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "grid_data": {},
            "stats": {
                "mouse_moves": 0,
                "mouse_clicks": 0,
                "keyboard_events": 0
            }
        }
    }))
}

// ============================================================================
// SECURITY ALERTS ENDPOINTS (NEW)
// ============================================================================

async fn list_security_alerts(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Query security_alerts table
    // TODO: Apply filters (severity, alert_type, time range)
    // TODO: Return latest alerts sorted by timestamp
    
    Json(json!({
        "success": true,
        "alerts": [],
        "total": 0
    }))
}

async fn list_device_alerts(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query security_alerts for specific device
    // TODO: Include both resolved and unresolved
    // TODO: Highlight process termination attempts
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "alerts": [],
        "unresolved_count": 0
    }))
}

async fn resolve_alert(
    State(_state): State<Arc<AppState>>,
    Path(alert_id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Update security_alerts SET resolved=true
    // TODO: Store resolution_notes from payload
    // TODO: Update updated_at timestamp
    
    Json(json!({
        "success": true,
        "alert_id": alert_id,
        "message": "Alert resolved successfully"
    }))
}

async fn record_termination_attempt(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract termination attempt details
    // TODO: Insert into process_termination_attempts table
    // TODO: Create CRITICAL security alert
    // TODO: Broadcast alert via WebSocket to dashboard
    
    Json(json!({
        "success": true,
        "message": "Termination attempt recorded and alerted",
        "alert_type": "PROCESS_TERMINATION_ATTEMPTED",
        "severity": "CRITICAL"
    }))
}

// JWT Verification Middleware
async fn verify_jwt_middleware(
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, String> {
    // For now, allow all protected routes (JWT verification will be enhanced in Phase 3.5)
    // The actual verification will be done in handler functions with State access
    Ok(next.run(req).await)
}

fn serialize_device(device: crate::postgres_db::Device, config: &RuntimeConfig) -> serde_json::Value {
    let online = device.last_seen > Utc::now() - Duration::seconds(config.online_threshold_seconds.max(1));
    let stale = !online;

    json!({
        "id": device.id,
        "device_id": device.device_id,
        "hostname": device.hostname,
        "nickname": device.nickname,
        "mac_address": device.mac_address.unwrap_or_else(|| "Unknown".to_string()),
        "created_at": device.created_at.to_rfc3339(),
        "last_seen": device.last_seen.to_rfc3339(),
        "online": online,
        "stale": stale,
        "status": if online { "online" } else { "offline" }
    })
}

// NEW ENDPOINTS FOR DASHBOARD

async fn get_overview(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_overview().await {
        Ok(overview) => Json(json!({
            "success": true,
            "data": {
                "devices_today": overview.devices_today,
                "active_time": overview.active_time,
                "idle_time": overview.idle_time,
                "idle_pct": format!("{:.1}%", overview.idle_pct),
                "keys_today": overview.keys_today,
                "mouse_moves_today": overview.mouse_moves_today,
                "mouse_clicks_today": overview.mouse_clicks_today,
            }
        })),
        Err(e) => {
            tracing::error!("Failed to fetch overview: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch overview data"
            }))
        }
    }
}

async fn get_top_apps(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Get top apps from last 7 days
    match state.db.get_top_apps(7).await {
        Ok(apps) => {
            let apps_json: Vec<serde_json::Value> = apps
                .into_iter()
                .map(|app| {
                    json!({
                        "app_name": app.app_name,
                        "total_duration_seconds": app.total_duration_seconds,
                        "total_duration_hours": format!("{:.2}", app.total_duration_seconds as f64 / 3600.0),
                    })
                })
                .collect();

            Json(json!({
                "success": true,
                "data": apps_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch top apps: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch top apps",
                "data": []
            }))
        }
    }
}

// SSE Stream endpoint
async fn stream_events(
    State(state): State<Arc<AppState>>,
) -> axum::response::sse::Sse<impl futures::stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let db = state.db.clone();
    let stream_poll_interval_ms = state.config.stream_poll_interval_ms.max(250);
    let stream_fetch_limit = state.config.stream_fetch_limit.max(50);
    let stream_max_devices = state.config.stream_max_devices.max(1);
    
    // Create a stream that emits events periodically
    let stream = stream::repeat_with(move || {
        let db = db.clone();
        async move {
            // Bounded poll interval to avoid flooding clients
            tokio::time::sleep(std::time::Duration::from_millis(stream_poll_interval_ms)).await;
            
            // Fetch a bounded window to prevent unbounded memory/cpu during bursts
            match db
                .get_activity_logs_filtered(None, None, None, Some(stream_fetch_limit))
                .await
            {
                Ok(logs) => {
                    if logs.is_empty() {
                        return Ok(axum::response::sse::Event::default());
                    }

                    // Group logs by device_id, keeping most recent
                    let mut device_logs: std::collections::HashMap<Uuid, &crate::postgres_db::ActivityLog> = std::collections::HashMap::new();
                    
                    for log in &logs {
                        device_logs.entry(log.device_id)
                            .and_modify(|existing| {
                                if log.timestamp > existing.timestamp {
                                    *existing = log;
                                }
                            })
                            .or_insert(log);
                    }

                    // Build a combined event with all device activities
                    let activities: Vec<serde_json::Value> = device_logs
                        .into_iter()
                        .take(stream_max_devices)
                        .map(|(_, log)| {
                            let is_idle = log.app_name.to_lowercase().contains("idle");
                            json!({
                                "device_id": log.device_id.to_string(),
                                "app": log.app_name,
                                "title": log.window_title,
                                "is_idle": is_idle,
                                "is_live": true,
                                "last_seen": log.timestamp.to_rfc3339(),
                            })
                        })
                        .collect();

                    let event_data = json!({
                        "activities": activities,
                        "timestamp": Utc::now().to_rfc3339(),
                    });

                    match axum::response::sse::Event::default()
                        .json_data(event_data) 
                    {
                        Ok(event) => Ok(event),
                        Err(_) => Ok(axum::response::sse::Event::default())
                    }
                }
                Err(e) => {
                    tracing::error!("Error fetching logs for stream: {}", e);
                    Ok(axum::response::sse::Event::default())
                }
            }
        }
    })
    .buffered(1);

    axum::response::sse::Sse::new(Box::pin(stream))
}
