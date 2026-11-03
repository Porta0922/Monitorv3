use axum::Router;
use std::sync::Arc;
use crate::api::AppState;
pub fn router()->Router<Arc<AppState>> {crate::human_sessions::router()}
