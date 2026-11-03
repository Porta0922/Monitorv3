use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use hyper::{Body, Request, Response, StatusCode};
use std::{convert::Infallible, time::Duration};
use tokio::{net::TcpListener, task::JoinHandle};

static LOCAL_RESOURCES: std::sync::LazyLock<std::sync::Mutex<serde_json::Value>> = std::sync::LazyLock::new(||std::sync::Mutex::new(serde_json::Value::Null));
pub fn update_local_resources(metrics: serde_json::Value) {
    if let Ok(mut value)=LOCAL_RESOURCES.lock() {*value=serde_json::json!({"observed_at":chrono::Utc::now(),"data":metrics});}
}
pub struct WebState {
    pub hostname: String,
    pub version: String,
    pub device_id: String,
    pub connected: Arc<AtomicBool>,
    pub events_today: Arc<AtomicU64>,
    pub cache_pending: Arc<AtomicU64>,
    pub auth_token: String,
    pub local_status: std::sync::Mutex<serde_json::Value>,
}

impl WebState {
    pub fn new(hostname: String, version: String, device_id: String) -> Self {
        let auth_token = std::env::var("WEB_UI_TOKEN")
            .unwrap_or_else(|_| {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let token: String = (0..32).map(|_| {
                    let idx = rng.gen_range(0..36);
                    if idx < 10 { (b'0' + idx) as char } else { (b'a' + idx - 10) as char }
                }).collect();
                token
            });
        Self {
            hostname,
            version,
            device_id,
            connected: Arc::new(AtomicBool::new(false)),
            events_today: Arc::new(AtomicU64::new(0)),
            cache_pending: Arc::new(AtomicU64::new(0)),
            auth_token,
            local_status: std::sync::Mutex::new(serde_json::Value::Null),
        }
    }
}

pub fn spawn_web_server(state: Arc<WebState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let bind = std::env::var("WEB_BIND").unwrap_or_else(|_| "127.0.0.1:9876".to_string());
        let address=match bind.parse::<std::net::SocketAddr>() {
            Ok(address) if address.ip().is_loopback()=>address,
            _=>{tracing::error!("Local view requires a literal loopback WEB_BIND address");return;}
        };
        match TcpListener::bind(address).await {
            Ok(listener) => {
                tracing::info!("Web UI started on http://{}", bind);
                if let Err(error) = serve(listener, state).await {
                    tracing::warn!(%error, "Web UI listener failed");
                }
            }
            Err(error) => tracing::warn!(%error, %bind, "Failed to start Web UI"),
        }
    })
}

async fn serve(listener: TcpListener, state: Arc<WebState>) -> std::io::Result<()> {
    // Dropping the supervisor-owned listener cancels every active HTTP connection,
    // including clients that never finish sending headers or a request body.
    let mut connections = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            result = listener.accept(), if connections.len() < 32 => {
                let (socket, _) = result?;
                let state = state.clone();
                connections.spawn(async move {
                    let handler = hyper::service::service_fn(move |request| {
                        let state = state.clone();
                        async move { Ok::<_, Infallible>(handle_request(request, state).await) }
                    });
                    let connection = hyper::server::conn::Http::new()
                        .http1_only(true).serve_connection(socket, handler);
                    let _ = tokio::time::timeout(Duration::from_secs(30), connection).await;
                });
            }
            _ = connections.join_next(), if !connections.is_empty() => {}
        }
    }
}

fn response(status: StatusCode, content_type: &str, body: impl Into<Body>) -> Response<Body> {
    Response::builder().status(status).header("Content-Type", content_type)
        .header("Cache-Control","no-store")
        .header("X-Content-Type-Options","nosniff")
        .header("Referrer-Policy","no-referrer")
        .header("Content-Security-Policy","default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")
        .body(body.into()).expect("Static response headers")
}

fn local_origin(request: &Request<Body>) -> bool {
    let Some(host)=request.headers().get(hyper::header::HOST).and_then(|h|h.to_str().ok()) else {return false;};
    let Ok(authority)=host.parse::<hyper::http::uri::Authority>() else {return false;};
    let host_name=authority.host().trim_start_matches('[').trim_end_matches(']');
    if host_name!="localhost" && !host_name.parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback()) {return false;}
    if request.headers().get("sec-fetch-site").and_then(|h|h.to_str().ok()).is_some_and(|s|!matches!(s,"same-origin"|"none")) {return false;}
    if let Some(origin)=request.headers().get(hyper::header::ORIGIN) {
        if origin.to_str().ok()!=Some(format!("http://{host}").as_str()) {return false;}
    }
    true
}

async fn handle_request(request: Request<Body>, state: Arc<WebState>) -> Response<Body> {
    if !local_origin(&request) {return response(StatusCode::FORBIDDEN,"text/plain","Local same-origin access required");}
    if request.method()!=hyper::Method::GET {return response(StatusCode::METHOD_NOT_ALLOWED,"application/json",r#"{"error":"Read-only local view"}"#);}
    match request.uri().path() {
        "/"=>response(StatusCode::OK,"text/html; charset=utf-8",include_str!("../ui/index.html")),
        "/app.css"=>response(StatusCode::OK,"text/css; charset=utf-8",include_str!("../ui/app.css")),
        "/app.js"=>response(StatusCode::OK,"text/javascript; charset=utf-8",include_str!("../ui/app.js")),
        "/api/local"=> {
            let resources=LOCAL_RESOURCES.lock().map(|v|v.clone()).unwrap_or(serde_json::Value::Null);
            let status=state.local_status.lock().map(|v|v.clone()).unwrap_or(serde_json::Value::Null);
            json_response(serde_json::json!({"hostname":state.hostname,"version":state.version,
                "queried_at":chrono::Utc::now(),"resources":resources,"agent":status,
                "capabilities":crate::runtime::capabilities()}).to_string())
        },
        "/api/status"=> {
            let authorized=request.headers().get(hyper::header::AUTHORIZATION).and_then(|v|v.to_str().ok())
                .is_some_and(|v|v==format!("Bearer {}",state.auth_token));
            if authorized {api_status(&state)} else {response(StatusCode::UNAUTHORIZED,"application/json",r#"{"error":"unauthorized"}"#)}
        },
        _=>response(StatusCode::NOT_FOUND,"text/plain","Not found"),
    }
}

fn json_response(body: String) -> Response<Body> {
    response(StatusCode::OK, "application/json; charset=utf-8", body)
}

fn api_status(state: &WebState) -> Response<Body> {
    let connected = state.connected.load(Ordering::Relaxed);
    let events = state.events_today.load(Ordering::Relaxed);
    let pending = state.cache_pending.load(Ordering::Relaxed);

    let json = serde_json::json!({
        "connected": connected,
        "hostname": state.hostname,
        "version": state.version,
        "events_since_start": events,
        "cache_pending": pending,
    })
    .to_string();

    json_response(json)
}

#[cfg(test)]
mod web_lifecycle_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    fn state() -> Arc<WebState> {
        let mut state = WebState::new("test".into(), "test".into(), "test".into());
        state.auth_token = "test-token".into();
        Arc::new(state)
    }

    #[tokio::test]
    async fn shutdown_closes_listener_and_incomplete_request() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(serve(listener, state()));
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        // First prove this connection is accepted and kept alive.
        socket.write_all(b"GET /api/status HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test-token\r\n\r\n").await.unwrap();
        let mut bytes = [0; 4096];
        let count = tokio::time::timeout(Duration::from_secs(2), socket.read(&mut bytes)).await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&bytes[..count]).contains("200 OK"));
        socket.write_all(b"POST /api/help HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test-token\r\nContent-Length: 1000\r\n\r\n{").await.unwrap();
        tokio::task::yield_now().await;
        server.abort(); let _ = server.await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop { match socket.read(&mut bytes).await { Ok(0) | Err(_) => break, _ => {} } }
        }).await.expect("Active connection must close on shutdown");
        let _rebound = TcpListener::bind(address).await.expect("Listener must release port");
    }

    #[tokio::test]
    async fn authorization_is_exact_and_body_is_bounded() {
        for uri in ["/api/status?x=token=test-token", "/api/status?token=test-token-extra"] {
            let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
            assert_eq!(handle_request(request, state()).await.status(), StatusCode::UNAUTHORIZED);
        }
        let request = Request::builder().uri("/api/status?token=test-token").body(Body::empty()).unwrap();
        assert_eq!(handle_request(request, state()).await.status(), StatusCode::OK);
        let request = Request::builder().method("POST").uri("/api/help")
            .header("Authorization", "Bearer test-token").body(Body::from(vec![b'a'; 16385])).unwrap();
        assert_eq!(handle_request(request, state()).await.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
