// REST API routes using Axum
use axum::{Router, routing::{get, patch}, extract::{State, Request}, middleware::{self, Next}, response::{Response, IntoResponse}, http::{StatusCode, Method}};
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

pub(crate) async fn valid_agent_token(state: &AppState, headers: &axum::http::HeaderMap) -> bool {
    crate::agent_identity::authenticate(state,headers).await.is_some()
}

async fn authorize(State(state): State<Arc<AppState>>, mut request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let path = path.strip_prefix("/api/").map(|p| format!("/{p}")).unwrap_or_else(|| path.to_owned());
    if matches!(path.as_str(),"/agent/enroll"|"/agent/renew") && !crate::agent_enrollment::allow_attempt(){return (StatusCode::TOO_MANY_REQUESTS,[("retry-after","60")]).into_response();}
    if matches!(path.as_str(), "/health" | "/ready" | "/auth/login" | "/agent/enroll") {
        return next.run(request).await;
    }
    let session=match crate::human_sessions::authenticate(&state,request.headers()).await {
        Ok(value)=>value,Err(_)=>return (StatusCode::SERVICE_UNAVAILABLE,axum::Json(serde_json::json!({"error":"Authentication unavailable"}))).into_response()
    };
    if let Some(session)=&session {
        if !matches!(*request.method(),Method::GET|Method::HEAD|Method::OPTIONS) && !crate::human_sessions::csrf_valid(request.headers(),session){return (StatusCode::FORBIDDEN,axum::Json(serde_json::json!({"error":"Invalid request verification"}))).into_response();}
        let internal=match state.auth.issue_role_token(&session.claims.sub,session.claims.role,1){Ok(token)=>token,Err(_)=>return StatusCode::SERVICE_UNAVAILABLE.into_response()};
        request.headers_mut().insert(axum::http::header::AUTHORIZATION,format!("Bearer {internal}").parse().expect("Generated token header"));
        request.extensions_mut().insert(session.clone());
    }
    let principal = if let Some(session)=&session {Some(session.claims.clone())} else if !crate::human_sessions::production() {
        request.headers().get(axum::http::header::AUTHORIZATION).and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).and_then(|token|state.auth.verify_token(token).ok())
    }else{None};
    if let Some(principal)=&principal {if !crate::human_sessions::api_budget(&principal.sub){return (StatusCode::TOO_MANY_REQUESTS,[("retry-after","60")]).into_response();}}
    let admin = principal.as_ref().is_some_and(|p|p.role==crate::auth::Role::Admin);
    let operator = principal.as_ref().is_some_and(|p|matches!(p.role,crate::auth::Role::Admin|crate::auth::Role::Operator));
    let agent_path=path.starts_with("/agent/") || matches!(path.as_str(),"/devices/register"|"/logs/ingest");
    let identity=if agent_path {crate::agent_identity::authenticate(&state,request.headers()).await} else {None};
    let agent=identity.is_some();
    let allowed = if path=="/auth/security" || path.starts_with("/auth/users/") || path.starts_with("/agent-credentials/") || path.starts_with("/administration/") || path.starts_with("/maintenance/") || path=="/audit" { admin }
    else if path.starts_with("/auth/") { session.is_some() }
    else if path=="/export/csv" || (path.starts_with("/devices/") && path.ends_with("/sessions/export")) { operator } else if path.starts_with("/agent/") {
        if path == "/agent/commands" && request.method() == Method::POST { admin } else { agent }
    } else if request.method() == Method::POST && matches!(path.as_str(), "/devices/register" | "/logs/ingest") {
        admin || agent
    } else if request.method()==Method::GET {
        principal.is_some()
    } else if path=="/security/investigations" || path.starts_with("/security/investigations/")
        || ((path.starts_with("/alerts/") || path.starts_with("/security/alerts/")) && path.ends_with("/resolve")) {
        operator
    } else {
        admin
    };
    if !allowed {
        if let Some(principal)=&principal {let _=tokio::time::timeout(std::time::Duration::from_millis(500),crate::human_sessions::audit(&state,&principal.sub,"access.denied",&path,"forbidden")).await;}
        return (if principal.is_some() {StatusCode::FORBIDDEN} else {StatusCode::UNAUTHORIZED}, axum::Json(serde_json::json!({"error":"Insufficient permissions or invalid credentials"}))).into_response();
    }
    if !admin {
        if let Some(crate::agent_identity::Identity::Device(device))=identity {
            let denied=|| (StatusCode::FORBIDDEN,axum::Json(serde_json::json!({"error":"Credential is not authorized for this device"}))).into_response();
            if matches!(path.as_str(),"/agent/policy"|"/agent/commands") {
                let params=match axum::extract::Query::<std::collections::HashMap<String,String>>::try_from_uri(request.uri()) {Ok(p)=>p,Err(_)=>return denied()};
                if !crate::agent_identity::matches_device(crate::agent_identity::Identity::Device(device),params.get("device_id").map(String::as_str)){return denied();}
            }
            if path.starts_with("/agent/commands/") && path.ends_with("/ack") {
                let command=path.strip_prefix("/agent/commands/").and_then(|v|v.strip_suffix("/ack")).and_then(|v|uuid::Uuid::parse_str(v).ok());
                let owner=match command {Some(id)=>sqlx::query_scalar::<_,uuid::Uuid>("SELECT device_id FROM agent_commands WHERE id=$1").bind(id).fetch_optional(&state.db.pool).await.ok().flatten(),None=>None};
                if owner!=Some(device){return denied();}
            }
            if request.method()==Method::POST && matches!(path.as_str(),"/devices/register"|"/logs/ingest") {
                let (parts,body)=request.into_parts();
                let bytes=match axum::body::to_bytes(body,2*1024*1024).await {Ok(b)=>b,Err(_)=>return StatusCode::PAYLOAD_TOO_LARGE.into_response()};
                let value:serde_json::Value=match serde_json::from_slice(&bytes){Ok(v)=>v,Err(_)=>return StatusCode::BAD_REQUEST.into_response()};
                if !crate::agent_identity::matches_device(crate::agent_identity::Identity::Device(device),value.get("device_id").and_then(|v|v.as_str())){return denied();}
                request=Request::from_parts(parts,axum::body::Body::from(bytes));
            }
        }
    }
    // Include only a validated device UUID in the audit target; never retain bodies,
    // query strings, credentials, window titles or operator-supplied reasons here.
    let mut audit_target=path.clone();
    if principal.is_some() {
        let mut device_id=axum::extract::Query::<std::collections::HashMap<String,String>>::try_from_uri(request.uri()).ok().and_then(|q|q.get("device_id").and_then(|s|uuid::Uuid::parse_str(s).ok()));
        if !matches!(*request.method(),Method::GET|Method::HEAD|Method::OPTIONS) {
            let (parts,body)=request.into_parts();
            let bytes=match axum::body::to_bytes(body,2*1024*1024).await {Ok(bytes)=>bytes,Err(_)=>return StatusCode::PAYLOAD_TOO_LARGE.into_response()};
            if let Ok(value)=serde_json::from_slice::<serde_json::Value>(&bytes){device_id=value.get("device_id").and_then(|v|v.as_str()).and_then(|s|uuid::Uuid::parse_str(s).ok()).or(device_id);}
            request=Request::from_parts(parts,axum::body::Body::from(bytes));
        }
        if let Some(device)=device_id{audit_target=format!("{path} device:{device}");}
    }
    let sensitive=principal.is_some() && (!matches!(*request.method(),Method::GET|Method::HEAD|Method::OPTIONS) || path=="/export/csv" || path.ends_with("/sessions/export") || path=="/audit");
    let actor=principal.as_ref().map(|p|p.sub.clone()).unwrap_or_default();
    let action=format!("http.{}",request.method());
    if sensitive && crate::human_sessions::audit(&state,&actor,&action,&audit_target,"requested").await.is_err(){return StatusCode::SERVICE_UNAVAILABLE.into_response();}
    let mut response=next.run(request).await;
    if sensitive && crate::human_sessions::audit(&state,&actor,&action,&audit_target,&format!("status_{}",response.status().as_u16())).await.is_err(){tracing::error!("Sensitive operation finished but audit completion failed");response.headers_mut().insert("x-monitor-audit-incomplete","true".parse().unwrap());}
    if let Some(principal)=&principal{response.headers_mut().insert("cache-control","no-store".parse().unwrap());let role=serde_json::to_value(principal.role).unwrap();response.headers_mut().insert("x-monitor-role",role.as_str().unwrap().parse().unwrap());}
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    #[tokio::test]
    async fn credential_management_is_admin_only_for_both_prefixes() {
        let state=Arc::new(AppState {auth:AuthManager::new("route-test"),db:Database{pool:PgPoolOptions::new().acquire_timeout(std::time::Duration::from_millis(100)).connect_lazy("postgres://test:test@localhost/test").unwrap()},config:RuntimeConfig::from_env(),rabbitmq_url:String::new()});
        let app=create_router(state.clone());
        for role in [crate::auth::Role::Viewer,crate::auth::Role::Operator] {
            let token=state.auth.issue_role_token("limited",role,1).unwrap();
            for prefix in ["","/api"] {
                for method in ["GET","POST","DELETE"] {
                    let path=if method=="DELETE" {"/agent-credentials/00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002"} else {"/agent-credentials/00000000-0000-0000-0000-000000000001"};
                    let result=app.clone().oneshot(Request::builder().method(method).uri(format!("{prefix}{path}")).header("Authorization",format!("Bearer {token}")).body(Body::empty()).unwrap()).await.unwrap();
                    assert_eq!(result.status(),StatusCode::FORBIDDEN);
                }
            }
        }
    }
    #[tokio::test]
    async fn api_prefix_and_parameters_are_routed_before_database_access() {
        let state = Arc::new(AppState {
            auth: AuthManager::new("route-test"),
            db: Database {
                pool: PgPoolOptions::new()
                    .acquire_timeout(std::time::Duration::from_millis(100)).connect_lazy("postgres://test:test@localhost/test").unwrap(),
            },
            config: RuntimeConfig::from_env(),
            rabbitmq_url: String::new(),
        });
        let token = state.auth.issue_token("admin", 1).unwrap();
        let app = create_router(state);
        for prefix in ["", "/api"] {
            for path in ["/resources/not-a-uuid", "/devices/not-a-uuid"] {
                let response = app.clone().oneshot(Request::builder()
                    .uri(format!("{prefix}{path}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty()).unwrap()).await.unwrap();
                assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{prefix}{path}");
            }
            let response = app.clone().oneshot(Request::builder()
                .uri(format!("{prefix}/auth/login"))
                .body(Body::empty()).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
            let response = app.clone().oneshot(Request::builder().uri(format!("{prefix}/ready"))
                .body(Body::empty()).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        }
    }
    #[tokio::test]
    async fn all_data_routes_require_jwt_and_agent_credentials_are_scoped() {
        let mut config = RuntimeConfig::from_env();
        config.agent_auth_token = "agent-test-secret".into();
        let state = Arc::new(AppState {
            auth: AuthManager::new("route-test"),
            db: Database { pool: PgPoolOptions::new().acquire_timeout(std::time::Duration::from_millis(100)).connect_lazy("postgres://test:test@localhost/test").unwrap() },
            config, rabbitmq_url: String::new(),
        });
        let expired = state.auth.issue_token("admin", -1).unwrap();
        let valid = state.auth.issue_token("admin", 1).unwrap();
        let app = create_router(state);
        for prefix in ["", "/api"] {
            for path in ["/devices", "/devices/no-id", "/logs", "/inventory/apps", "/usb", "/wifi", "/security", "/heatmaps", "/overview", "/top_apps", "/history", "/history_hourly_programs", "/hourly", "/available_dates", "/active_vs_idle", "/live_devices", "/metrics/summary", "/resources/no-id", "/resources/no-id/daily", "/maintenance/status", "/resources_peaks", "/audit", "/export/csv", "/alerts"] {
                for token in ["", "invalid", expired.as_str()] {
                    let response = app.clone().oneshot(Request::builder().uri(format!("{prefix}{path}"))
                        .header("Authorization", format!("Bearer {token}"))
                        .header("x-agent-token", "agent-test-secret").body(Body::empty()).unwrap()).await.unwrap();
                    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{prefix}{path}");
                }
            }
            for (method, path) in [("POST", "/devices/register"), ("POST", "/logs/ingest"), ("PATCH", "/devices/no-id"), ("POST", "/alerts"), ("PATCH", "/alerts/1/resolve"), ("POST", "/agent/commands"), ("POST", "/agent/commands/no-id/ack")] {
                let response = app.clone().oneshot(Request::builder().method(method).uri(format!("{prefix}{path}"))
                    .body(Body::empty()).unwrap()).await.unwrap();
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            }
            for path in ["/agent/policy", "/agent/osquery-policy"] {
                let response = app.clone().oneshot(Request::builder().uri(format!("{prefix}{path}"))
                    .header("Authorization", format!("Bearer {valid}")).body(Body::empty()).unwrap()).await.unwrap();
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
                let response = app.clone().oneshot(Request::builder().uri(format!("{prefix}{path}"))
                    .header("x-agent-token", "agent-test-secret").body(Body::empty()).unwrap()).await.unwrap();
                assert_eq!(response.status(), if path=="/agent/policy" {StatusCode::SERVICE_UNAVAILABLE} else {StatusCode::OK});
            }
        }
    }

}




pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new();
    let cors = cors
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PATCH,
            axum::http::Method::PUT,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::HeaderName::from_static("x-agent-token"),
        ]);

    let routes = Router::new()
        // Domain nests
        .nest("/agent-credentials",crate::agent_identity::router())
        .nest("/administration",crate::domains::administration::router())
        .nest("/devices", crate::domains::device::routes::router())
        .nest("/logs", crate::domains::activity::routes::router())
        .nest("/inventory", crate::domains::inventory::routes::router())
        .nest("/usb", crate::domains::usb::routes::router())
        .nest("/security", crate::domains::security::routes::router())
        .nest("/heatmaps", crate::domains::keystroke::routes::router())
        .nest("/agent", crate::domains::agent::routes::router())
        .nest("/wifi", crate::domains::wifi::routes::router())
        .nest("/auth", crate::domains::auth::routes::router())
        // Dashboard routes at top level (the frontend calls them without a prefix)
        .route("/overview", get(crate::domains::dashboard::routes::get_overview))
        .route("/top_apps", get(crate::domains::dashboard::routes::get_top_apps))
        .route("/history", get(crate::domains::dashboard::routes::get_history))
        .route("/history_hourly_programs", get(crate::domains::dashboard::routes::get_history_hourly_programs))
        .route("/hourly", get(crate::domains::dashboard::routes::get_hourly))
        .route("/available_dates", get(crate::domains::dashboard::routes::get_available_dates))
        .route("/active_vs_idle", get(crate::domains::dashboard::routes::get_active_vs_idle))
        .route("/live_devices", get(crate::domains::dashboard::routes::get_live_devices))
        .route("/metrics/summary", get(crate::domains::dashboard::routes::get_metrics_summary))
        .route("/devices/:device_id/telemetry", get(crate::domains::dashboard::telemetry::get_state))
        .route("/resources/:device_id", get(crate::domains::dashboard::routes::get_device_resources))
        .route("/maintenance/dead-letters", get(crate::domains::dashboard::dead_letters::status))
        .route("/maintenance/dead-letters/:queue/replay", axum::routing::post(crate::domains::dashboard::dead_letters::replay))
        .route("/maintenance/storage", get(crate::domains::dashboard::maintenance::storage))
        .route("/maintenance/status", get(crate::domains::dashboard::maintenance::get_status))
        .route("/resources/:device_id/daily", get(crate::domains::dashboard::daily::get_daily_metrics))
        .route("/resources_peaks", get(crate::domains::dashboard::routes::get_resource_peaks))
        .route("/audit", get(crate::domains::dashboard::routes::get_audit_events))
        .route("/export/csv", get(crate::domains::dashboard::routes::export_csv))
        .route("/health", get(crate::domains::dashboard::routes::health))
        .route("/ready", get(crate::domains::dashboard::routes::ready))
        // Alerts at top level (frontend calls /alerts not /security/alerts)
        .route("/alerts", get(crate::domains::security::routes::list_security_alerts).post(crate::domains::security::routes::create_security_alert))
        .route("/alerts/:id/resolve", patch(crate::domains::security::routes::resolve_alert));

    // Keep legacy agent URLs while exposing the dashboard API under /api.
    Router::new()
        .nest("/api", routes.clone())
        .merge(routes)
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, authorize))
        .layer(axum::extract::DefaultBodyLimit::max(2*1024*1024))
        .layer(cors)
}
