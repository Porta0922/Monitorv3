use axum::{extract::{Query, State, Path}, response::IntoResponse, routing::{get, post, patch}, Json, Router};
use serde_json::json;
use std::sync::Arc;
use crate::api::AppState;
use crate::domains::shared::{parse_iso_date, ActivityLogFilters, DateLimitQuery, TzQuery, LiveDevicesQuery, ActiveIdleQuery, format_duration, parse_time_bounds, serialize_device};
use uuid::Uuid;
use super::models::*;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // ADD ROUTES HERE
}

