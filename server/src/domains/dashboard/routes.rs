use axum::{Router, extract::{State, Path, Query}, routing::get, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;
use crate::domains::shared::{parse_iso_date, format_duration};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/overview", get(get_overview))
        .route("/top_apps", get(get_top_apps))
        .route("/history", get(get_history))
        .route("/history_hourly_programs", get(get_history_hourly_programs))
        .route("/hourly", get(get_hourly))
        .route("/available_dates", get(get_available_dates))
        .route("/active_vs_idle", get(get_active_vs_idle))
        .route("/live_devices", get(get_live_devices))
        .route("/metrics/summary", get(get_metrics_summary))
        .route("/resources/{}", get(get_device_resources))
        .route("/resources_peaks", get(get_resource_peaks))
        .route("/audit", get(get_audit_events))
        .route("/export/csv", get(export_csv))
        .route("/health", get(health))
}

#[derive(Deserialize, Default)]
pub struct OverviewQuery {
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Deserialize, Default)]
pub struct HistoryQuery {
    pub device_id: String,
    pub date: Option<String>,
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Deserialize, Default)]
pub struct HourlyQuery {
    pub device_id: String,
    pub date: Option<String>,
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Deserialize, Default)]
pub struct AvailableDatesQuery {
    pub device_id: Option<String>,
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Deserialize, Default)]
pub struct ActiveVsIdleQuery {
    pub days: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct LiveDevicesQuery {
    pub live_only: Option<bool>,
    pub limit: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct ResourceQuery {
    pub date: Option<String>,
    pub limit: Option<i64>,
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Deserialize, Default)]
pub struct ResourcePeaksQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct AuditQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct ExportCsvQuery {
    pub device_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Deserialize, Default)]
pub struct TopAppsQuery {
    pub days: Option<i64>,
}

pub async fn get_overview(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_overview().await {
        Ok(overview) => Json(json!({
            "success": true,
            "data": {
                "devices_today": overview.devices_today,
                "active_time": overview.active_time,
                "idle_time": overview.idle_time,
                "idle_pct": overview.idle_pct,
                "keys_today": overview.keys_today,
                "mouse_moves_today": overview.mouse_moves_today,
                "mouse_clicks_today": overview.mouse_clicks_today
            }
        })),
        Err(e) => {
            tracing::error!("Failed to fetch overview: {}", e);
            Json(json!({ "success": false, "data": {} }))
        }
    }
}

pub async fn get_top_apps(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TopAppsQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(1);
    match state.db.get_top_apps(days).await {
        Ok(apps) => {
            let data: Vec<serde_json::Value> = apps.into_iter().map(|app| {
                json!({
                    "app_name": app.app_name,
                    "total_duration_seconds": app.total_duration_seconds,
                    "total_duration_hours": format_duration(app.total_duration_seconds)
                })
            }).collect();
            Json(json!({ "success": true, "data": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch top apps: {}", e);
            Json(json!({ "success": false, "data": [] }))
        }
    }
}

pub async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return Json(json!({ "success": false, "error": "invalid device_id" })),
    };
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    let today = chrono::Utc::now().date_naive();
    let date = parse_iso_date(query.date.as_deref()).unwrap_or(today);

    match state.db.get_device_history_for_date(device_uuid, date, tz_offset).await {
        Ok(rows) => {
            let history: Vec<serde_json::Value> = rows.into_iter().map(|(app_name, window_title, total_seconds, count)| {
                json!({
                    "app_name": app_name,
                    "window_title": window_title,
                    "total_seconds": total_seconds,
                    "count": count
                })
            }).collect();
            Json(json!({ "success": true, "history": history }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch history: {}", e);
            Json(json!({ "success": false, "history": [] }))
        }
    }
}

pub async fn get_history_hourly_programs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HourlyQuery>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return Json(json!({ "success": false, "error": "invalid device_id" })),
    };
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    let today = chrono::Utc::now().date_naive();
    let date = parse_iso_date(query.date.as_deref()).unwrap_or(today);

    match state.db.get_device_programs_by_hour_for_date(device_uuid, date, tz_offset).await {
        Ok(rows) => {
            let groups: Vec<serde_json::Value> = rows.into_iter().map(|(hour, app_name, window_title, total_seconds, count)| {
                json!({
                    "hour": hour,
                    "app_name": app_name,
                    "window_title": window_title,
                    "total_seconds": total_seconds,
                    "count": count
                })
            }).collect();
            Json(json!({ "success": true, "groups": groups }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch history hourly programs: {}", e);
            Json(json!({ "success": false, "groups": [] }))
        }
    }
}

pub async fn get_hourly(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HourlyQuery>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return Json(json!({ "success": false, "error": "invalid device_id" })),
    };
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    let today = chrono::Utc::now().date_naive();
    let date = parse_iso_date(query.date.as_deref()).unwrap_or(today);

    match state.db.get_device_hourly_for_date(device_uuid, date, tz_offset).await {
        Ok(rows) => {
            let hourly: Vec<serde_json::Value> = rows.into_iter().map(|(hour, active_seconds, idle_seconds)| {
                json!({
                    "hour": hour,
                    "active_seconds": active_seconds,
                    "idle_seconds": idle_seconds
                })
            }).collect();
            Json(json!({ "success": true, "hourly": hourly }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch hourly data: {}", e);
            Json(json!({ "success": false, "hourly": [] }))
        }
    }
}

pub async fn get_available_dates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AvailableDatesQuery>,
) -> impl IntoResponse {
    let device_uuid = query.device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);

    match state.db.get_available_dates(device_uuid, 60, tz_offset).await {
        Ok(dates) => {
            let date_strings: Vec<String> = dates.into_iter().map(|d| d.format("%Y-%m-%d").to_string()).collect();
            Json(json!({ "success": true, "dates": date_strings }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch available dates: {}", e);
            Json(json!({ "success": false, "dates": [] }))
        }
    }
}

pub async fn get_active_vs_idle(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ActiveVsIdleQuery>,
) -> impl IntoResponse {
    let days = query.days.unwrap_or(7);
    let since = chrono::Utc::now() - chrono::Duration::days(days);

    match state.db.get_active_vs_idle_since(since).await {
        Ok(rows) => {
            let data: Vec<serde_json::Value> = rows.into_iter().map(|(device_id, active, idle)| {
                json!({
                    "device_id": device_id.to_string(),
                    "active_seconds": active,
                    "idle_seconds": idle
                })
            }).collect();
            Json(json!({ "success": true, "data": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch active vs idle: {}", e);
            Json(json!({ "success": false, "data": [] }))
        }
    }
}

pub async fn get_live_devices(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LiveDevicesQuery>,
) -> impl IntoResponse {
    match state.db.get_live_devices_activity().await {
        Ok(rows) => {
            let mut devices: Vec<serde_json::Value> = rows.into_iter().map(|d| {
                json!({
                    "device_id": d.device_id.to_string(),
                    "app_name": d.app_name,
                    "window_title": d.window_title,
                    "duration_seconds": d.duration_seconds,
                    "timestamp": d.timestamp.to_rfc3339()
                })
            }).collect();

            if let Some(limit) = query.limit {
                devices.truncate(limit.max(0) as usize);
            }

            Json(json!({ "success": true, "devices": devices }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch live devices: {}", e);
            Json(json!({ "success": false, "devices": [] }))
        }
    }
}

pub async fn get_metrics_summary(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_operational_metrics(state.config.online_threshold_seconds, state.config.stale_threshold_seconds).await {
        Ok(metrics) => Json(json!({
            "success": true,
            "metrics": {
                "devices_total": metrics.devices_total,
                "devices_online": metrics.devices_online,
                "devices_stale": metrics.devices_stale,
                "activities_last_hour": metrics.activities_last_hour,
                "input_rows_today": metrics.input_rows_today,
                "newest_activity_age_seconds": metrics.newest_activity_age_seconds
            }
        })),
        Err(e) => {
            tracing::error!("Failed to fetch operational metrics: {}", e);
            Json(json!({ "success": false, "metrics": {} }))
        }
    }
}

pub async fn get_device_resources(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(query): Query<ResourceQuery>,
) -> impl IntoResponse {
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    let today = chrono::Utc::now().date_naive();
    let date = parse_iso_date(query.date.as_deref()).unwrap_or(today);
    let limit = query.limit.unwrap_or(2880);

    match state.db.get_device_resource_metrics_for_date(device_id, date, tz_offset, limit).await {
        Ok(metrics) => {
            let data: Vec<serde_json::Value> = metrics.into_iter().map(|m| {
                json!({
                    "timestamp": m.timestamp.to_rfc3339(),
                    "cpu_percent": m.cpu_percent,
                    "memory_used_mb": m.memory_used_mb,
                    "memory_percent": m.memory_percent,
                    "top_process_name": m.top_process_name,
                    "top_process_cpu_percent": m.top_process_cpu_percent,
                    "top_process_memory_mb": m.top_process_memory_mb
                })
            }).collect();
            Json(json!({ "success": true, "metrics": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device resources: {}", e);
            Json(json!({ "success": false, "metrics": [] }))
        }
    }
}

pub async fn get_resource_peaks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ResourcePeaksQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20);

    match state.db.get_resource_peaks_today(limit).await {
        Ok(peaks) => {
            let data: Vec<serde_json::Value> = peaks.into_iter().map(|p| {
                json!({
                    "device_id": p.device_id.to_string(),
                    "peak_cpu_percent": p.peak_cpu_percent,
                    "peak_memory_percent": p.peak_memory_percent,
                    "last_cpu_percent": p.last_cpu_percent,
                    "last_memory_percent": p.last_memory_percent,
                    "top_process_name": p.top_process_name,
                    "top_process_cpu_percent": p.top_process_cpu_percent,
                    "top_process_memory_mb": p.top_process_memory_mb,
                    "last_seen": p.last_seen.to_rfc3339()
                })
            }).collect();
            Json(json!({ "success": true, "peaks": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch resource peaks: {}", e);
            Json(json!({ "success": false, "peaks": [] }))
        }
    }
}

pub async fn get_audit_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);

    match state.db.get_audit_events(limit).await {
        Ok(events) => {
            let data: Vec<serde_json::Value> = events.into_iter().map(|e| {
                json!({
                    "id": e.id,
                    "actor": e.actor,
                    "action": e.action,
                    "target": e.target,
                    "details": e.details,
                    "created_at": e.created_at.to_rfc3339()
                })
            }).collect();
            Json(json!({ "success": true, "events": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch audit events: {}", e);
            Json(json!({ "success": false, "events": [] }))
        }
    }
}

pub async fn export_csv(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExportCsvQuery>,
) -> impl IntoResponse {
    let device_uuid = query.device_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);

    let today = chrono::Utc::now().date_naive();
    let from = parse_iso_date(query.from.as_deref()).unwrap_or(today);
    let to = parse_iso_date(query.to.as_deref()).unwrap_or(today);

    match state.db.get_activity_logs_for_export(device_uuid, from, to, tz_offset).await {
        Ok(logs) => {
            let mut csv = String::from("device_id,app_name,window_title,duration_seconds,timestamp\n");
            for log in logs {
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    log.device_id,
                    log.app_name.replace(',', ";"),
                    log.window_title.replace(',', ";"),
                    log.duration_seconds,
                    log.timestamp.to_rfc3339()
                ));
            }
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8")],
                csv,
            )
        }
        Err(e) => {
            tracing::error!("Failed to export CSV: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                serde_json::to_string(&json!({ "success": false })).unwrap(),
            )
        }
    }
}

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
