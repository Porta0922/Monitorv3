use axum::{Router, extract::{State, Json}, routing::post, response::IntoResponse};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use crate::api::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    if body.username == admin_username && body.password == admin_password {
        match state.auth.issue_token(&body.username, 24) {
            Ok(token) => (StatusCode::OK, Json(json!({
                "token": token,
                "expires_in": 86400,
                "token_type": "Bearer"
            }))),
            Err(e) => {
                tracing::error!("Failed to issue token: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "failed to issue token" })))
            }
        }
    } else {
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "invalid credentials" })))
    }
}
