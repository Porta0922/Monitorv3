// REST API routes using Axum
use axum::{Router, routing::{get, patch, post}};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

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
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::any())
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::HeaderName::from_static("x-agent-token"),
        ]);

    Router::new()
        // Domain nests
        .nest("/devices", crate::domains::device::routes::router())
        .nest("/logs", crate::domains::activity::routes::router())
        .nest("/inventory", crate::domains::inventory::routes::router())
        .nest("/usb", crate::domains::usb::routes::router())
        .nest("/security", crate::domains::security::routes::router())
        .nest("/heatmaps", crate::domains::keystroke::routes::router())
        .nest("/agent", crate::domains::agent::routes::router())
        .nest("/wifi", crate::domains::wifi::routes::router())
        .nest("/auth", crate::domains::auth::routes::router())
        // Dashboard routes at top level (the frontend calls them without a prefix)
        .route("/overview", get(crate::domains::dashboard::routes::get_overview))
        .route("/top_apps", get(crate::domains::dashboard::routes::get_top_apps))
        .route("/history", get(crate::domains::dashboard::routes::get_history))
        .route("/history_hourly_programs", get(crate::domains::dashboard::routes::get_history_hourly_programs))
        .route("/hourly", get(crate::domains::dashboard::routes::get_hourly))
        .route("/available_dates", get(crate::domains::dashboard::routes::get_available_dates))
        .route("/active_vs_idle", get(crate::domains::dashboard::routes::get_active_vs_idle))
        .route("/live_devices", get(crate::domains::dashboard::routes::get_live_devices))
        .route("/metrics/summary", get(crate::domains::dashboard::routes::get_metrics_summary))
        .route("/resources/{}", get(crate::domains::dashboard::routes::get_device_resources))
        .route("/resources_peaks", get(crate::domains::dashboard::routes::get_resource_peaks))
        .route("/audit", get(crate::domains::dashboard::routes::get_audit_events))
        .route("/export/csv", get(crate::domains::dashboard::routes::export_csv))
        .route("/health", get(crate::domains::dashboard::routes::health))
        // Alerts at top level (frontend calls /alerts not /security/alerts)
        .route("/alerts", get(crate::domains::security::routes::list_security_alerts).post(crate::domains::security::routes::create_security_alert))
        .route("/alerts/{id}/resolve", patch(crate::domains::security::routes::resolve_alert))
        .with_state(state)
        .layer(cors)
}
