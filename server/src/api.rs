// REST API routes using Axum
use axum::Router;
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
    let cors = CorsLayer::permissive();

    Router::new()
        .nest("/devices", crate::domains::device::routes::router())
        .nest("/logs", crate::domains::activity::routes::router())
        .nest("/inventory", crate::domains::inventory::routes::router())
        .nest("/usb", crate::domains::usb::routes::router())
        .nest("/security", crate::domains::security::routes::router())
        .nest("/heatmaps", crate::domains::keystroke::routes::router())
        .nest("/agent", crate::domains::agent::routes::router())
        .with_state(state)
        .layer(cors)
}
