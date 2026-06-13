use axum::{extract::{Query, State, Path}, response::IntoResponse, routing::{get, post, patch}, Json, Router};
use serde_json::json;
use std::sync::Arc;
use crate::api::{AppState, parse_iso_date};
use uuid::Uuid;
use super::models::*;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // ADD ROUTES HERE
}

