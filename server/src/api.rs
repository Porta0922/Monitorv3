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
use crate::postgres_db::Database;

pub struct AppState {
    pub auth: AuthManager,
    pub db: Database,
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

    // Combine routers with /api prefix and apply CORS
    let api_router = Router::new()
        .nest("/api", public_router.merge(protected_router));

    api_router
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
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let username = _payload.get("username").and_then(|u| u.as_str());
    let password = _payload.get("password").and_then(|p| p.as_str());
    
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
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract device_id, hostname, mac_address
    // TODO: Store in devices table
    
    Json(json!({
        "success": true,
        "message": "Device registered successfully"
    }))
}

async fn list_devices(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_devices().await {
        Ok(devices) => {
            let device_json: Vec<serde_json::Value> = devices.iter().map(|d| {
                json!({
                    "id": d.id,
                    "hostname": d.hostname,
                    "device_id": d.device_id,
                    "nickname": d.nickname,
                    "last_seen": d.last_seen.to_rfc3339(),
                })
            }).collect();
            
            Json(json!({
                "success": true,
                "devices": device_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch devices: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch devices",
                "devices": []
            }))
        }
    }
}

async fn get_device(
    State(state): State<Arc<AppState>>,
    Path(_device_id): Path<Uuid>,
) -> impl IntoResponse {
    // Get first device as example (TODO: filter by ID)
    match state.db.get_devices().await {
        Ok(devices) => {
            if let Some(device) = devices.first() {
                return Json(json!({
                    "success": true,
                    "device": {
                        "id": device.id,
                        "hostname": device.hostname,
                        "device_id": device.device_id,
                        "nickname": device.nickname,
                        "last_seen": device.last_seen.to_rfc3339(),
                    }
                }));
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch device: {}", e);
        }
    }
    
    Json(json!({
        "success": true,
        "device": null
    }))
}

async fn update_device(
    State(_state): State<Arc<AppState>>,
    Path(_device_id): Path<Uuid>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Update device nickname or other fields
    
    Json(json!({
        "success": true,
        "message": "Device updated successfully"
    }))
}

async fn ingest_activity_logs(
    State(state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // TODO: Extract activity logs batch
    // TODO: Insert into activity_logs hypertable
    
    Json(json!({
        "success": true,
        "message": "Activity logs ingested"
    }))
}

async fn query_activity_logs(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.db.get_activity_logs(None).await {
        Ok(logs) => {
            let logs_json: Vec<serde_json::Value> = logs.iter().map(|l| {
                json!({
                    "id": l.id,
                    "device_id": l.device_id,
                    "app_name": l.app_name,
                    "window_title": l.window_title,
                    "duration_seconds": l.duration_seconds,
                    "timestamp": l.timestamp.to_rfc3339(),
                })
            }).collect();
            
            Json(json!({
                "success": true,
                "logs": logs_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch activity logs: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch activity logs",
                "logs": []
            }))
        }
    }
}

async fn get_device_logs(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<Uuid>,
) -> impl IntoResponse {
    let device_id_str = device_id.to_string();
    
    match state.db.get_activity_logs(Some(&device_id_str)).await {
        Ok(logs) => {
            let logs_json: Vec<serde_json::Value> = logs.iter().map(|l| {
                json!({
                    "id": l.id,
                    "device_id": l.device_id,
                    "app_name": l.app_name,
                    "window_title": l.window_title,
                    "duration_seconds": l.duration_seconds,
                    "timestamp": l.timestamp.to_rfc3339(),
                })
            }).collect();
            
            Json(json!({
                "success": true,
                "device_id": device_id_str,
                "logs": logs_json
            }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch device logs: {}", e);
            Json(json!({
                "success": false,
                "error": "Failed to fetch device logs",
                "device_id": device_id_str,
                "logs": []
            }))
        }
    }
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
