// REST API routes using Axum
use axum::{
    Router,
    routing::{get, post, patch},
    Json,
    extract::{Path, State},
    middleware::{self, Next},
    http::Method,
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, AllowOrigin, AllowHeaders};

use crate::auth::AuthManager;

pub struct AppState {
    pub auth: AuthManager,
    // Will add db connection pool here
}

pub fn create_router(state: Arc<AppState>) -> Router {
    // Configure CORS layer
    let cors = CorsLayer::permissive()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin
                .as_bytes()
                .eq(b"http://localhost:5173")
                || origin.as_bytes().eq(b"http://localhost:3000")
                || origin.as_bytes().eq(b"http://127.0.0.1:5173")
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS, Method::PATCH])
        .allow_headers(AllowHeaders::any());

    // Create the main router with protected endpoints
    let protected_router = Router::new()
        // Protected endpoints (require JWT)
        .route("/devices", get(list_devices))
        .route("/devices/:device_id", get(get_device))
        .route("/devices/:device_id", patch(update_device))
        .route("/logs/ingest", post(ingest_activity_logs))
        .route("/logs", get(query_activity_logs))
        .route("/logs/:device_id", get(get_device_logs))
        // TODO: .route("/inventory/apps", get(list_all_apps))
        // TODO: .route("/inventory/apps/:device_id", get(list_device_apps))
        
        // NEW: Input Heatmaps
        .route("/heatmaps/upload", post(upload_heatmap))
        .route("/heatmaps/:device_id", get(get_device_heatmaps))
        .route("/heatmaps/:device_id/current", get(get_current_heatmap))
        
        // NEW: Security Alerts
        .route("/alerts", get(list_security_alerts))
        .route("/alerts/:device_id", get(list_device_alerts))
        .route("/alerts/:alert_id/resolve", patch(resolve_alert))
        .route("/alerts/process-protection", post(record_termination_attempt))
        
        // Apply JWT middleware to protected routes only
        .layer(axum::middleware::from_fn(verify_jwt_middleware))
        .with_state(state.clone());

    // Public routes
    let public_router = Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(register_user))
        .route("/auth/login", post(login_user))
        .route("/devices/register", post(register_device))
        .with_state(state);

    // Combine routers and apply CORS
    public_router
        .merge(protected_router)
        .layer(cors)
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

// ============================================================================
// INPUT HEATMAP ENDPOINTS (NEW)
// ============================================================================

async fn upload_heatmap(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract heatmap data from payload
    // TODO: Validate heatmap format
    // TODO: Insert into input_activity_heatmaps hypertable
    // TODO: Update input_activity_daily_summary
    
    Json(json!({
        "success": true,
        "message": "Heatmap uploaded successfully",
        "heatmap_id": "HEATMAP-001"
    }))
}

async fn get_device_heatmaps(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query input_activity_heatmaps for specific device
    // TODO: Apply time filters (last 7 days, last 30 days, etc)
    // TODO: Optionally aggregate by hour/day
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "heatmaps": []
    }))
}

async fn get_current_heatmap(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Get most recent heatmap for device
    // TODO: Return with real-time visualization data
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "current_heatmap": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "grid_data": {},
            "stats": {
                "mouse_moves": 0,
                "mouse_clicks": 0,
                "keyboard_events": 0
            }
        }
    }))
}

// ============================================================================
// SECURITY ALERTS ENDPOINTS (NEW)
// ============================================================================

async fn list_security_alerts(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // TODO: Query security_alerts table
    // TODO: Apply filters (severity, alert_type, time range)
    // TODO: Return latest alerts sorted by timestamp
    
    Json(json!({
        "success": true,
        "alerts": [],
        "total": 0
    }))
}

async fn list_device_alerts(
    State(_state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    // TODO: Query security_alerts for specific device
    // TODO: Include both resolved and unresolved
    // TODO: Highlight process termination attempts
    
    Json(json!({
        "success": true,
        "device_id": device_id.to_string(),
        "alerts": [],
        "unresolved_count": 0
    }))
}

async fn resolve_alert(
    State(_state): State<Arc<AppState>>,
    Path(alert_id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Update security_alerts SET resolved=true
    // TODO: Store resolution_notes from payload
    // TODO: Update updated_at timestamp
    
    Json(json!({
        "success": true,
        "alert_id": alert_id,
        "message": "Alert resolved successfully"
    }))
}

async fn record_termination_attempt(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract termination attempt details
    // TODO: Insert into process_termination_attempts table
    // TODO: Create CRITICAL security alert
    // TODO: Broadcast alert via WebSocket to dashboard
    
    Json(json!({
        "success": true,
        "message": "Termination attempt recorded and alerted",
        "alert_type": "PROCESS_TERMINATION_ATTEMPTED",
        "severity": "CRITICAL"
    }))
}

// JWT Verification Middleware
async fn verify_jwt_middleware(
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, String> {
    // For now, allow all protected routes (JWT verification will be enhanced in Phase 3.5)
    // The actual verification will be done in handler functions with State access
    Ok(next.run(req).await)
}
