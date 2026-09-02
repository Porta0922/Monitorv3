use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use tiny_http::{Header, Response, Server, StatusCode};

pub struct WebState {
    pub hostname: String,
    pub version: String,
    pub device_id: String,
    pub connected: Arc<AtomicBool>,
    pub events_today: Arc<AtomicU64>,
    pub cache_pending: Arc<AtomicU64>,
    pub auth_token: String,
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
        }
    }
}

pub fn spawn_web_server(state: Arc<WebState>) {
    let auth_token = state.auth_token.clone();
    thread::spawn(move || {
        let bind = std::env::var("WEB_BIND").unwrap_or_else(|_| "0.0.0.0:9876".to_string());
        let server = match Server::http(&bind) {
            Ok(s) => {
                tracing::info!("Web UI started on http://{} (token: {})", &bind, &auth_token[..4]);
                s
            }
            Err(e) => {
                tracing::warn!("Failed to start Web UI on {}: {}", bind, e);
                return;
            }
        };

        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().as_str().to_string();

            // Check auth token on API endpoints
            if url.starts_with("/api/") {
                let url_has_token = url.contains(&format!("token={}", auth_token));
                let header_has_token = request.headers().iter().any(|h| {
                    let field_name = format!("{}", h.field);
                    let field_lower = field_name.to_lowercase();
                    if field_lower == "authorization" {
                        let val = format!("{}", h.value);
                        val == format!("Bearer {}", auth_token)
                    } else {
                        false
                    }
                });
                if !url_has_token && !header_has_token {
                    let _ = request.respond(
                        Response::from_string(r#"{"error":"unauthorized"}"#)
                            .with_status_code(StatusCode(401))
                            .with_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap())
                    );
                    continue;
                }
            }

            match (&method[..], &url[..]) {
                ("GET", "/") => {
                    let _ = request.respond(index_html(&state));
                }
                ("GET", url) if url.starts_with("/api/status") => {
                    let _ = request.respond(api_status(&state));
                }
                ("POST", url) if url.starts_with("/api/help") => {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    let _ = request.respond(api_help_body(&state, &body));
                }
                ("POST", url) if url.starts_with("/api/uninstall") => {
                    let _ = request.respond(api_uninstall());
                }
                _ => {
                    let _ = request.respond(
                        Response::from_string("404 Not Found")
                            .with_status_code(StatusCode(404))
                    );
                }
            };
        }
    });
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn index_html(state: &WebState) -> Response<Cursor<Vec<u8>>> {
    let connected = state.connected.load(Ordering::Relaxed);
    let events = state.events_today.load(Ordering::Relaxed);
    let pending = state.cache_pending.load(Ordering::Relaxed);
    let status_icon = if connected { "\u{1f7e2}" } else { "\u{1f534}" };
    let status_text = if connected { "Conectado" } else { "Desconectado" };
    let device_short = if state.device_id.len() > 8 { &state.device_id[..8] } else { &state.device_id };

    let hn = html_escape(&state.hostname);
    let ver = html_escape(&state.version);
    let did = html_escape(device_short);
    let st = html_escape(status_text);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>ActivityMonitor Agent</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:-apple-system,'Segoe UI',system-ui,sans-serif;background:#0d1117;color:#c9d1d9;display:flex;justify-content:center;align-items:center;min-height:100vh}}
.card{{background:#161b22;border:1px solid #30363d;border-radius:12px;padding:32px;max-width:480px;width:100%;margin:20px}}
h1{{font-size:20px;margin-bottom:20px;color:#58a6ff}}
.metric{{display:flex;justify-content:space-between;padding:10px 0;border-bottom:1px solid #21262d}}
.metric:last-child{{border-bottom:none}}
.label{{color:#8b949e}}
.value{{font-weight:600}}
.status{{display:inline-flex;align-items:center;gap:6px}}
h2{{font-size:16px;margin:24px 0 8px;color:#58a6ff}}
textarea{{width:100%;background:#0d1117;border:1px solid #30363d;border-radius:6px;color:#c9d1d9;padding:8px;min-height:80px;resize:vertical;margin-top:8px;font-family:inherit;font-size:14px}}
button{{background:#238636;color:#fff;border:none;border-radius:6px;padding:8px 16px;cursor:pointer;margin-top:8px;font-size:14px;font-weight:500}}
button:hover{{background:#2ea043}}button:disabled{{opacity:.6;cursor:not-allowed}}
#msg{{margin-top:8px;font-size:13px}}
.footer{{margin-top:24px;padding-top:12px;border-top:1px solid #21262d;font-size:11px;color:#484f58;text-align:center}}
</style>
</head>
<body>
<div class="card">
<h1>ActivityMonitor Agent</h1>
<div class="metric"><span class="label">Estado</span><span class="value status">{si} {st}</span></div>
<div class="metric"><span class="label">Hostname</span><span class="value">{hn}</span></div>
<div class="metric"><span class="label">Versión</span><span class="value">{ver}</span></div>
<div class="metric"><span class="label">Device ID</span><span class="value">{did}</span></div>
<div class="metric"><span class="label">Eventos Hoy</span><span class="value">{ev}</span></div>
<div class="metric"><span class="label">Cache Pendiente</span><span class="value">{cp}</span></div>
<h2>Solicitar Ayuda</h2>
<textarea id="helpMsg" placeholder="Describe tu problema..."></textarea>
<button onclick="sendHelp()" id="sendBtn">Enviar</button>
<div id="msg"></div>
<div class="footer">ActivityMonitor Enterprise v{ver}</div>
</div>
<script>
async function sendHelp(){{
const msg=document.getElementById('helpMsg').value;
if(!msg)return;
const btn=document.getElementById('sendBtn');
btn.disabled=true;
try{{
const res=await fetch('/api/help',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{message:msg}})}});
const data=await res.json();
document.getElementById('msg').textContent=data.success?'Enviado con éxito':'Error al enviar mensaje';
document.getElementById('msg').style.color=data.success?'#3fb950':'#f85149';
}}catch(e){{
document.getElementById('msg').textContent='Error de conexión';
document.getElementById('msg').style.color='#f85149';
}}
btn.disabled=false;
}}
</script>
</body></html>"#,
        si = status_icon,
        st = st,
        hn = hn,
        ver = ver,
        did = did,
        ev = events,
        cp = pending,
    );

    let ct = Header::from_bytes(b"Content-Type", b"text/html; charset=utf-8")
        .unwrap_or_else(|_| panic!("Invalid header"));
    Response::from_string(html).with_header(ct)
}

fn json_response(body: String) -> Response<Cursor<Vec<u8>>> {
    let ct = Header::from_bytes(b"Content-Type", b"application/json; charset=utf-8")
        .unwrap_or_else(|_| panic!("Invalid header"));
    Response::new(
        StatusCode(200),
        vec![ct],
        Cursor::new(body.into_bytes()),
        None,
        None,
    )
}

fn api_status(state: &WebState) -> Response<Cursor<Vec<u8>>> {
    let connected = state.connected.load(Ordering::Relaxed);
    let events = state.events_today.load(Ordering::Relaxed);
    let pending = state.cache_pending.load(Ordering::Relaxed);

    let json = serde_json::json!({
        "connected": connected,
        "hostname": state.hostname,
        "version": state.version,
        "events_today": events,
        "cache_pending": pending,
    })
    .to_string();

    json_response(json)
}

fn api_help_body(_state: &WebState, body: &str) -> Response<Cursor<Vec<u8>>> {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let message = parsed["message"].as_str().unwrap_or("");

    let success = if !message.is_empty() {
        tracing::info!("Help request from user: {}", message);
        true
    } else {
        false
    };

    let json = format!(r#"{{"success":{}}}"#, if success { "true" } else { "false" });
    json_response(json)
}

fn api_uninstall() -> Response<Cursor<Vec<u8>>> {
    match crate::uninstall::spawn_uninstall() {
        Ok(_) => {
            tracing::warn!("Uninstall requested from local Web UI. Launching self-uninstall.");
            json_response(r#"{"success":true,"message":"Uninstall launched"}"#.to_string())
        }
        Err(e) => json_response(
            serde_json::json!({ "success": false, "message": format!("Failed to launch uninstall: {}", e) })
                .to_string(),
        ),
    }
}
