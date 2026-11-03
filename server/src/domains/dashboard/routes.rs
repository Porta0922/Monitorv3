use axum::{Router, extract::{State, Path, Query}, routing::get, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;
use crate::domains::shared::{parse_iso_date, format_duration};

fn dashboard_json(value:serde_json::Value)->axum::response::Response {
 let status=if value["success"]==false {
  if value["error"].as_str().is_some_and(|s|s.starts_with("invalid")){axum::http::StatusCode::BAD_REQUEST}else{axum::http::StatusCode::INTERNAL_SERVER_ERROR}
 }else{axum::http::StatusCode::OK};
 (status,Json(value)).into_response()
}

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
        .route("/resources/:device_id", get(get_device_resources))
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
    let result=sqlx::query_scalar::<_,serde_json::Value>(r#"
      SELECT jsonb_build_object('devices_today',count(DISTINCT device_id),'active_time',sum(active_person_seconds),
       'idle_time',sum(idle_person_seconds),'keys_today',sum(keys_count),'mouse_moves_today',sum(mouse_moves_count),'mouse_clicks_today',sum(clicks_count),
       'idle_pct',100.0*sum(idle_person_seconds)/NULLIF(sum(active_person_seconds)+sum(idle_person_seconds),0),
       'timezone','UTC','metric_scope','person_session','available_samples',COALESCE(sum(available_samples),0),
       'unavailable_samples',COALESCE(sum(unavailable_samples),0),'unknown_samples',COALESCE(sum(unknown_samples),0))
      FROM session_activity_combined WHERE day=(now() AT TIME ZONE 'UTC')::date
    "#).fetch_one(&state.db.pool).await;
    match result {Ok(value)=>dashboard_json(json!({"success":true,"data":value})).into_response(),Err(error)=>{tracing::error!(%error,"Overview unavailable");(axum::http::StatusCode::INTERNAL_SERVER_ERROR,dashboard_json(json!({"success":false,"error":"Overview unavailable"}))).into_response()}}
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
            dashboard_json(json!({ "success": true, "data": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch top apps: {}", e);
            dashboard_json(json!({ "success": false, "data": [] }))
        }
    }
}

pub async fn get_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return dashboard_json(json!({ "success": false, "error": "invalid device_id" })),
    };
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    if !(-840..=840).contains(&tz_offset){return dashboard_json(json!({"success":false,"error":"invalid UTC offset"}));}
    let today = chrono::Utc::now().date_naive();
    let date = match query.date.as_deref(){None=>today,Some(v)=>match parse_iso_date(Some(v)){Some(d)=>d,None=>return dashboard_json(json!({"success":false,"error":"invalid date"}))}};

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
            dashboard_json(json!({ "success": true, "history": history }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch history: {}", e);
            dashboard_json(json!({ "success": false, "history": [] }))
        }
    }
}

pub async fn get_history_hourly_programs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HourlyQuery>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return dashboard_json(json!({ "success": false, "error": "invalid device_id" })),
    };
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    if !(-840..=840).contains(&tz_offset){return dashboard_json(json!({"success":false,"error":"invalid UTC offset"}));}
    let today = chrono::Utc::now().date_naive();
    let date = match query.date.as_deref(){None=>today,Some(v)=>match parse_iso_date(Some(v)){Some(d)=>d,None=>return dashboard_json(json!({"success":false,"error":"invalid date"}))}};

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
            dashboard_json(json!({ "success": true, "groups": groups }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch history hourly programs: {}", e);
            dashboard_json(json!({ "success": false, "groups": [] }))
        }
    }
}

pub async fn get_hourly(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HourlyQuery>,
) -> impl IntoResponse {
    let device_uuid = match Uuid::parse_str(&query.device_id) {
        Ok(u) => u,
        Err(_) => return dashboard_json(json!({ "success": false, "error": "invalid device_id" })),
    };
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    if !(-840..=840).contains(&tz_offset){return dashboard_json(json!({"success":false,"error":"invalid UTC offset"}));}
    let today = chrono::Utc::now().date_naive();
    let date = match query.date.as_deref(){None=>today,Some(v)=>match parse_iso_date(Some(v)){Some(d)=>d,None=>return dashboard_json(json!({"success":false,"error":"invalid date"}))}};

    match state.db.get_device_hourly_for_date(device_uuid, date, tz_offset).await {
        Ok(rows) => {
            let hourly: Vec<serde_json::Value> = rows.into_iter().map(|(hour, active_seconds, idle_seconds)| {
                json!({
                    "hour": hour,
                    "active_seconds": active_seconds,
                    "idle_seconds": idle_seconds
                })
            }).collect();
            dashboard_json(json!({ "success": true, "hourly": hourly }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch hourly data: {}", e);
            dashboard_json(json!({ "success": false, "hourly": [] }))
        }
    }
}

pub async fn get_available_dates(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AvailableDatesQuery>,
) -> impl IntoResponse {
    let device_uuid = match query.device_id.as_deref().map(Uuid::parse_str).transpose(){Ok(v)=>v,Err(_)=>return dashboard_json(json!({"success":false,"error":"invalid device_id"}))};
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    if !(-840..=840).contains(&tz_offset){return dashboard_json(json!({"success":false,"error":"invalid UTC offset"}));}

    match state.db.get_available_dates(device_uuid, 60, tz_offset).await {
        Ok(dates) => {
            let date_strings: Vec<String> = dates.into_iter().map(|d| d.format("%Y-%m-%d").to_string()).collect();
            dashboard_json(json!({ "success": true, "dates": date_strings }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch available dates: {}", e);
            dashboard_json(json!({ "success": false, "dates": [] }))
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
            dashboard_json(json!({ "success": true, "data": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch active vs idle: {}", e);
            dashboard_json(json!({ "success": false, "data": [] }))
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

            dashboard_json(json!({ "success": true, "devices": devices }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch live devices: {}", e);
            dashboard_json(json!({ "success": false, "devices": [] }))
        }
    }
}

pub async fn get_metrics_summary(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_operational_metrics(state.config.online_threshold_seconds, state.config.stale_threshold_seconds).await {
        Ok(metrics) => dashboard_json(json!({
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
            dashboard_json(json!({ "success": false, "metrics": {} }))
        }
    }
}

pub async fn get_device_resources(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Query(query): Query<ResourceQuery>,
) -> impl IntoResponse {
    let tz_offset = query.tz_offset_minutes.unwrap_or(0);
    if !(-840..=840).contains(&tz_offset){return dashboard_json(json!({"success":false,"error":"invalid UTC offset"}));}
    let today = chrono::Utc::now().date_naive();
    let date = match query.date.as_deref(){None=>today,Some(v)=>match parse_iso_date(Some(v)){Some(d)=>d,None=>return dashboard_json(json!({"success":false,"error":"invalid date"}))}};
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
            dashboard_json(json!({ "success": true, "metrics": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device resources: {}", e);
            dashboard_json(json!({ "success": false, "metrics": [] }))
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
            dashboard_json(json!({ "success": true, "peaks": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch resource peaks: {}", e);
            dashboard_json(json!({ "success": false, "peaks": [] }))
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
            dashboard_json(json!({ "success": true, "events": data }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch audit events: {}", e);
            dashboard_json(json!({ "success": false, "events": [] }))
        }
    }
}

pub(crate) fn csv_cell(value:&str)->String {
    let leading=value.trim_start_matches(|c:char|c.is_whitespace()||c.is_control());
    let safe=if leading.starts_with(['=','+','-','@']) || value.starts_with(['\t','\r','\n']) {format!("'{value}")}else{value.to_owned()};
    format!("\"{}\"",safe.replace('"',"\"\""))
}

pub async fn export_csv(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExportCsvQuery>,
) -> impl IntoResponse {
    let invalid=|message:&str| (axum::http::StatusCode::BAD_REQUEST,
        [(axum::http::header::CONTENT_TYPE,"application/json")],json!({"error":message}).to_string());
    let device_uuid=match query.device_id.as_deref().map(Uuid::parse_str).transpose(){Ok(v)=>v,Err(_)=>return invalid("Invalid device_id")};
    let tz_offset=query.tz_offset_minutes.unwrap_or(0);
    if !(-840..=840).contains(&tz_offset){return invalid("UTC offset must be between -840 and 840 minutes");}
    let today=(chrono::Utc::now()+chrono::Duration::minutes(tz_offset as i64)).date_naive();
    let from=match query.from.as_deref(){None=>today,Some(v)=>match parse_iso_date(Some(v)){Some(d)=>d,None=>return invalid("Invalid from date")}};
    let to=match query.to.as_deref(){None=>today,Some(v)=>match parse_iso_date(Some(v)){Some(d)=>d,None=>return invalid("Invalid to date")}};
    if from>to || (to-from).num_days()>365 || to.succ_opt().is_none(){return invalid("Export range must be ordered and at most 366 days");}

    match state.db.get_activity_logs_for_export(device_uuid, from, to, tz_offset).await {
        Ok(logs) => {
            let mut csv = String::from("device_id,app_name,window_title,duration_seconds,timestamp\n");
            for log in logs {
                csv.push_str(&format!(
                    "{},{},{},{},{}\n",
                    log.device_id,
                    csv_cell(&log.app_name),
                    csv_cell(&log.window_title),
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
    dashboard_json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Availability requires completed queue setup and a responsive database.
pub async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let consumer_ready = crate::rabbitmq_consumer::CONSUMER_READY.load(std::sync::atomic::Ordering::Acquire);
    let database_ready = consumer_ready && matches!(tokio::time::timeout(
        std::time::Duration::from_millis(state.config.readiness_timeout_ms), state.db.ping()
    ).await, Ok(Ok(())));
    if database_ready {
        (axum::http::StatusCode::OK, dashboard_json(json!({"status":"ready"})))
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, dashboard_json(json!({"status":"not_ready"})))
    }
}

#[cfg(test)]
mod csv_security_tests {
 #[tokio::test]
 async fn export_filters_never_expand_invalid_scope() {
  use super::*;
  let state=Arc::new(crate::api::AppState{auth:crate::auth::AuthManager::new("test"),db:crate::postgres_db::Database{pool:sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://unused:unused@localhost/unused").unwrap()},config:crate::config::RuntimeConfig::from_env(),rabbitmq_url:String::new()});
  for query in [
   ExportCsvQuery{device_id:Some("not-a-uuid".into()),..Default::default()},
   ExportCsvQuery{from:Some("2026-02-30".into()),..Default::default()},
   ExportCsvQuery{to:Some("invalid".into()),..Default::default()},
   ExportCsvQuery{from:Some("2026-02-02".into()),to:Some("2026-02-01".into()),..Default::default()},
   ExportCsvQuery{from:Some("2020-01-01".into()),to:Some("2026-01-01".into()),..Default::default()},
   ExportCsvQuery{tz_offset_minutes:Some(i32::MAX),..Default::default()}
  ] {
   let response=export_csv(State(state.clone()),Query(query)).await.into_response();
   assert_eq!(response.status(),axum::http::StatusCode::BAD_REQUEST);
  }
 }
 use super::csv_cell;
 #[test] fn spreadsheet_formulas_and_quotes_are_escaped() {
  assert_eq!(csv_cell("=1+1"), "\"'=1+1\"");
  assert_eq!(csv_cell("  @SUM(A1)"), "\"'  @SUM(A1)\"");
  assert_eq!(csv_cell("a,\"b\""), "\"a,\"\"b\"\"\"");
  assert_eq!(csv_cell("ordinary"), "\"ordinary\"");
 }
}
