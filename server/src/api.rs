// REST API routes using Axum
use axum::{
    Router,
    routing::{get, post, patch},
    Json,
    extract::{Path, Query, State},
    middleware::Next,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{Duration, Utc};
use tower_http::cors::CorsLayer;
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
    let cors = CorsLayer::permissive();

    Router::new()
        .nest("/devices", crate::domains::device::routes::router())
        .nest("/logs", crate::domains::activity::routes::router())
        .nest("/inventory", crate::domains::inventory::routes::router())
        .nest("/usb", crate::domains::usb::routes::router())
        .nest("/security", crate::domains::security::routes::router())
        .nest("/heatmaps", crate::domains::keystroke::routes::router())
        .with_state(state)
        .layer(cors)
}
