use axum::Router;
use std::sync::Arc;
use crate::api::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // ADD ROUTES HERE
}

