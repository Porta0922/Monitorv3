// REST API routes using Axum
use axum::{
    Router,
    routing::{get, post, patch},
    Json,
    extract::{Path, State},
    middleware::{self, Next},
    http::Request,
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;
use tower::Layer;

use crate::auth::AuthManager;

pub struct AppState {
    pub auth: AuthManager,
    // Will add db connection pool here
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Public endpoints
        .route("/health", get(health_check))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route("/devices/register", post(register_device))
        
        // Protected endpoints (require JWT)
        .route("/devices", get(list_devices))
        .route("/devices/:device_id", get(get_device))
        .route("/devices/:device_id", patch(update_device))
        .route("/logs/ingest", post(ingest_activity_logs))
        .route("/logs", get(query_activity_logs))
        .route("/logs/:device_id", get(get_device_logs))
        .route("/inventory/apps", get(list_all_apps))
        .route("/inventory/apps/:device_id", get(list_device_apps))
        
        .layer(middleware::from_fn(verify_jwt_middleware))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "ActivityMonitor Server",
        "version": "0.1.0"
    }))
}

async fn register_user(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract username, password from payload
    // TODO: Hash password using AuthManager
    // TODO: Store in database
    
    Json(json!({
        "success": true,
        "message": "User registered successfully"
    }))
}

async fn login_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let username = payload.get("username").and_then(|u| u.as_str());
    let password = payload.get("password").and_then(|p| p.as_str());
    
    if let (Some(username), Some(password)) = (username, password) {
        // TODO: Fetch user from database
        // TODO: Verify password hash
        
        if let Ok(token) = state.auth.issue_token(username, 24) {
            return Json(json!({
                "success": true,
                "token": token,
                "expires_in": 86400
            })).into_response();
        }
    }
    
    Json(json!({
        "success": false,
        "error": "Invalid credentials"
    })).into_response()
}

async fn register_device(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract device_id, hostname, mac_address
    // TODO: Store in devices table
    
    Json(json!({
        "success": true,
        "message": "Device registered successfully"
    }))
}

async fn list_devices(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Query all devices from database
    
    Json(json!({
        "success": true,
        "devices": []
    }))
}

async fn get_device(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query specific device
    
    Json(json!({
        "success": true,
        "device": null
    }))
}

async fn update_device(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Update device nickname or other fields
    
    Json(json!({
        "success": true,
        "message": "Device updated successfully"
    }))
}

async fn ingest_activity_logs(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract activity logs batch
    // TODO: Insert into activity_logs hypertable
    
    Json(json!({
        "success": true,
        "message": "Activity logs ingested"
    }))
}

async fn query_activity_logs(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Query activity_logs with filters (device_id, app_name, date range)
    
    Json(json!({
        "success": true,
        "logs": []
    }))
}

async fn get_device_logs(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query activity_logs for specific device
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "logs": []
    }))
}

async fn list_all_apps(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Query all apps from app_inventory across all devices
    
    Json(json!({
        "success": true,
        "apps": []
    }))
}

async fn list_device_apps(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query apps for specific device
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "apps": []
    }))
}

// JWT Verification Middleware
async fn verify_jwt_middleware<B>(
    req: Request<B>,
    next: Next,
) -> Result<Response, String> {
    // For now, skip JWT verification (implement in Phase 3.5)
    // TODO: Extract Authorization header
    // TODO: Verify JWT token
    // TODO: Add user to request extensions
    
    Ok(next.run(req).await)
}
