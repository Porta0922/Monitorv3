// ============================================================
//  manager - Consola local de control de flota (web en 127.0.0.1)
//  - Tabla de maquinas (hostname, version, last_seen, estado)
//  - Releases publicados en GitHub + latest esperado
//  - Publicar agente/remover (ejecuta scripts\publish.ps1)
//
//  Config: manager.config.json (server_url) junto al exe o en la raiz del repo.
//  Token:  github_token.txt en la raiz del repo.
// ============================================================

use anyhow::Context;
use axum::{Json, Router, extract::State, routing::{get, post}};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use tokio::{process::Command, sync::Mutex};

const GITHUB_API: &str = "https://api.github.com/repos/Porta0922/Monitorv3";
const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    gh_token: String,
    server_url: String,
    repo_root: PathBuf,
    publish_lock: Arc<Mutex<()>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let repo_root = PathBuf::from(REPO_ROOT);
    let token = load_token(&repo_root).await;
    let server_url = load_server_url(&repo_root).await?;

    let client = reqwest::Client::builder()
        .user_agent("activity-monitor-manager")
        .build()
        .context("fallo al crear reqwest client")?;

    let state = AppState {
        client,
        gh_token: token,
        server_url,
        repo_root,
        publish_lock: Arc::new(Mutex::new(())),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/health", get(api_health))
        .route("/api/devices", get(api_devices))
        .route("/api/releases", get(api_releases))
        .route("/api/publish", post(api_publish))
        .with_state(state);

    let addr = "127.0.0.1:7272";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Manager web en http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn load_token(repo_root: &PathBuf) -> String {
    let path = repo_root.join("github_token.txt");
    match tokio::fs::read_to_string(&path).await {
        Ok(t) => t.trim().to_string(),
        Err(e) => {
            tracing::warn!("No se pudo leer github_token.txt ({e}): publicar quedara deshabilitado.");
            String::new()
        }
    }
}

async fn load_server_url(repo_root: &PathBuf) -> anyhow::Result<String> {
    if let Ok(v) = std::env::var("MANAGER_SERVER_URL") {
        if !v.is_empty() {
            return Ok(v.trim_end_matches('/').to_string());
        }
    }
    let mut candidates = vec![
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("manager.config.json"))),
        Some(repo_root.join("manager.config.json")),
    ];
    candidates.retain(|c| c.is_some());
    for cand in candidates.into_iter().flatten() {
        if let Ok(raw) = tokio::fs::read_to_string(&cand).await {
            if let Ok(cfg) = serde_json::from_str::<ManagerConfig>(&raw) {
                tracing::info!("Config desde {}: server_url={}", cand.display(), cfg.server_url);
                return Ok(cfg.server_url.trim_end_matches('/').to_string());
            }
        }
    }
    tracing::warn!("Sin configuración; usando default http://localhost:3000");
    Ok("http://localhost:3000".to_string())
}

#[derive(Deserialize)]
struct ManagerConfig {
    server_url: String,
}

async fn index() -> axum::response::Html<&'static str> {
    axum::response::Html(INDEX_HTML)
}

async fn api_health(State(st): State<AppState>) -> Json<Value> {
    let reachable = st.client.get(format!("{}/devices", st.server_url)).send().await;
    let server_reachable = match &reachable {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };
    Json(json!({
        "manager": "ok",
        "server_url": st.server_url,
        "server_reachable": server_reachable,
    }))
}

async fn api_devices(State(st): State<AppState>) -> Json<Value> {
    match st
        .client
        .get(format!("{}/devices", st.server_url))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(body) => Json(json!({ "ok": true, "server_url": st.server_url, "body": body })),
            Err(e) => Json(json!({ "ok": false, "error": format!("json: {e}") })),
        },
        Ok(resp) => Json(json!({ "ok": false, "error": format!("server HTTP {}", resp.status()) })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn api_releases(State(st): State<AppState>) -> Json<Value> {
    let headers = gh_headers(&st.gh_token);
    let list = st
        .client
        .get(format!("{GITHUB_API}/releases"))
        .query(&[("per_page", "10")])
        .headers(headers.clone())
        .send()
        .await;
    let latest = st
        .client
        .get(format!("{GITHUB_API}/releases/latest"))
        .headers(headers)
        .send()
        .await;

    let parse = |resp: Result<reqwest::Response, reqwest::Error>| async move {
        match resp {
            Ok(r) if r.status().is_success() => r.json::<Value>().await,
            Ok(_) => Ok(json!({ "error": "not found" })),
            Err(e) => Ok(json!({ "error": e.to_string() })),
        }
    };
    let list_val = parse(list).await.unwrap_or_else(|e| json!({ "error": e.to_string() }));
    let latest_val = parse(latest).await.unwrap_or_else(|e| json!({ "error": e.to_string() }));

    Json(json!({
        "token_presente": !st.gh_token.is_empty(),
        "latest": latest_val,
        "releases": list_val,
    }))
}

fn gh_headers(token: &str) -> reqwest::header::HeaderMap {
    use reqwest::header::{AUTHORIZATION, USER_AGENT};
    let mut h = reqwest::header::HeaderMap::new();
    h.insert(USER_AGENT, "activity-monitor-manager".parse().unwrap());
    if !token.is_empty() {
        h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    }
    h
}

#[derive(Deserialize)]
struct PublishBody {
    role: String,
}

async fn api_publish(State(st): State<AppState>, Json(body): Json<PublishBody>) -> Json<Value> {
    if st.gh_token.is_empty() {
        return Json(json!({ "ok": false, "output": "No hay github_token.txt (o esta vacio)." }));
    }
    if body.role != "agent" && body.role != "remover" {
        return Json(json!({ "ok": false, "output": "role debe ser agent o remover." }));
    }
    let _guard = st.publish_lock.lock().await;

    let ps = st.repo_root.join("scripts").join("publish.ps1");
    if !ps.exists() {
        return Json(json!({ "ok": false, "output": format!("No existe {}", ps.display()) }));
    }

    let child = match Command::new("powershell")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&ps)
        .arg("-Role")
        .arg(&body.role)
        .output()
        .await
    {
        Ok(c) => c,
        Err(e) => return Json(json!({ "ok": false, "output": format!("no se pudo ejecutar powershell: {e}") })),
    };
    let out = String::from_utf8_lossy(&child.stdout).to_string();
    let err = String::from_utf8_lossy(&child.stderr).to_string();
    let combined = if err.trim().is_empty() { out } else { format!("{out}\n[stderr]\n{err}") };

    Json(json!({
        "ok": child.status.success(),
        "output": combined,
    }))
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="es">
<head>
<meta charset="utf-8">
<title>Monitor v3 - Control de Flota</title>
<style>
  :root { color-scheme: dark; }
  body { font-family: system-ui, sans-serif; margin: 0; background: #0f172a; color: #e2e8f0; }
  header { display: flex; align-items: center; gap: 16px; padding: 12px 20px; background:#1e293b; border-bottom: 1px solid #334155; flex-wrap: wrap; }
  h1 { font-size: 18px; margin: 0; }
  .badge { padding: 3px 10px; border-radius: 999px; font-size: 12px; background: #334155; }
  .badge.ok { background: #14532d; color: #bbf7d0; }
  .badge.warn { background: #713f12; color: #fde68a; }
  .badge.err { background: #7f1d1d; color: #fecaca; }
  button { background: #2563eb; border: 0; color: #fff; padding: 7px 14px; border-radius: 8px; cursor: pointer; font-size: 13px; }
  button:disabled { background: #475569; cursor: not-allowed; }
  button.danger { background: #dc2626; }
  main { padding: 20px; display: grid; grid-template-columns: 1fr; gap: 20px; }
  .card { background: #1e293b; border: 1px solid #334155; border-radius: 12px; padding: 16px; }
  .card h2 { margin: 0 0 12px; font-size: 14px; text-transform: uppercase; letter-spacing: .5px; color: #94a3b8; }
  table { width: 100%; border-collapse: collapse; font-size: 13px; }
  th, td { text-align: left; padding: 8px 10px; border-bottom: 1px solid #334155; }
  th { color: #94a3b8; font-weight: 600; }
  .state { padding: 2px 9px; border-radius: 999px; font-size: 11px; }
  .state.online { background: #14532d; color: #bbf7d0; }
  .state.stale { background: #713f12; color: #fde68a; }
  .state.off { background: #7f1d1d; color: #fecaca; }
  .mismatch { color: #fca5a5; font-weight: 700; }
  #modal { display:none; position: fixed; inset: 0; background: rgba(0,0,0,.6); z-index: 10; align-items:center; justify-content:center; }
  #modal.open { display:flex; }
  #modal .box { background:#1e293b; border:1px solid #475569; border-radius:12px; width: min(720px, 92vw); max-height: 80vh; display:flex; flex-direction:column; }
  #modal header { border-bottom:1px solid #334155; }
  #modal pre { margin:0; padding:14px; overflow:auto; font-size:12px; white-space:pre-wrap; }
  .muted { color: #64748b; font-size: 12px; }
</style>
</head>
<body>
<header>
  <h1>Monitor v3 &middot; Control de Flota</h1>
  <span id="badge-server" class="badge">server ...</span>
  <span id="badge-token" class="badge">token ...</span>
  <button onclick="refreshAll()">&#8635; Refrescar</button>
  <span style="flex:1"></span>
  <span class="muted" id="last-updated"></span>
</header>
<main>
  <div class="card">
    <h2>Flota &middot; <span id="flota-count" class="muted"></span></h2>
    <table><thead>
      <tr><th>Hostname</th><th>Device ID</th><th>Version</th><th>Esperado</th><th>Ultima senal</th><th>Estado</th></tr>
    </thead><tbody id="flota-body"><tr><td colspan="6" class="muted">cargando...</td></tr></tbody></table>
  </div>
  <div class="card">
    <h2>Releases (GitHub)</h2>
    <table><thead>
      <tr><th>Tag</th><th>Fecha</th><th>Asset</th><th>Estatus</th></tr>
    </thead><tbody id="releases-body"><tr><td colspan="4" class="muted">cargando...</td></tr></tbody></table>
    <div style="margin-top:10px; display:flex; gap:8px;">
      <button id="pub-agent" onclick="publish('agent')">Publicar AGENTE (nuevo latest)</button>
      <button id="pub-remover" class="danger" onclick="publish('remover')">Publicar REMOVER</button>
    </div>
    <div class="muted" style="margin-top:8px">Versión que se publica = la del Cargo.toml del crate correspondiente.</div>
  </div>
</main>

<div id="modal">
  <div class="box">
    <header><h1 id="modal-title">Publicando...</h1><button onclick="closeModal()" style="margin-left:auto; background:#475569">Cerrar</button></header>
    <pre id="modal-output">...</pre>
  </div>
</div>

<script>
let expected = null;
function openModal(title){ document.getElementById('modal-title').textContent = title; document.getElementById('modal-output').textContent = '...'; document.getElementById('modal').classList.add('open'); }
function closeModal(){ document.getElementById('modal').classList.remove('open'); }
function setBadge(id, txt, kind){ const b = document.getElementById(id); b.textContent = txt; b.className = 'badge' + (kind ? ' ' + kind : ''); }
function age(s){ const d = new Date(s); if(isNaN(d)) return '?'; const diff = Date.now() - d.getTime(); const m = Math.floor(diff/60000); if(m < 1) return 'hace <1 min'; if(m < 60) return 'hace ' + m + ' min'; const h = Math.floor(m/60); return 'hace ' + h + ' h ' + (m%60) + ' min'; }

async function refreshDevices(){
  try {
    const r = await fetch('/api/devices'); const j = await r.json();
    if(!j.ok) { setBadge('badge-server','server ERROR','err'); return; }
    setBadge('badge-server', j.server_url, 'ok');
    const devs = (j.body && j.body.devices) || [];
    document.getElementById('flota-count').textContent = devs.length + ' maquinas';
    const tbody = document.getElementById('flota-body');
    if(!devs.length){ tbody.innerHTML = '<tr><td colspan="6" class="muted">sin maquinas registradas</td></tr>'; return; }
    tbody.innerHTML = devs.map(d => {
      const seen = new Date(d.last_seen).getTime();
      const diffMin = (Date.now() - seen)/60000;
      const st = diffMin < 3 ? ['online','online'] : diffMin < 30 ? ['stale','stale'] : ['off','off'];
      const v = d.version ? d.version : '?';
      const exp = expected ? 'v' + expected : '';
      const mis = expected && v !== '?' && v !== 'v' + expected;
      return '<tr><td>'+d.hostname+'</td><td class="muted">'+ (d.device_id||'').slice(0,8) +'...</td>' +
        '<td'+(mis?' class="mismatch"':'')+'>'+v+'</td><td class="muted">'+exp+'</td>' +
        '<td class="muted">'+age(d.last_seen)+'</td><td><span class="state '+st[0]+'">'+st[1]+'</span></td></tr>';
    }).join('');
  } catch(e){ setBadge('badge-server','server OFF','err'); }
}

async function refreshReleases(){
  try {
    const r = await fetch('/api/releases'); const j = await r.json();
    setBadge('badge-token', j.token_presente ? 'token OK' : 'token FALTA', j.token_presente ? 'ok' : 'err');
    const list = Array.isArray(j.releases) ? j.releases : (j.releases && j.releases.error ? [] : []);
    if(j.latest && j.latest.tag_name){ expected = j.latest.tag_name.replace(/^v/,''); } else { expected = null; }
    const tbody = document.getElementById('releases-body');
    if(!list.length){ tbody.innerHTML = '<tr><td colspan="4" class="muted">sin releases</td></tr>'; return; }
    tbody.innerHTML = list.map(x => {
      const a = x.assets && x.assets[0];
      const size = a ? (a.size/1024/1024).toFixed(2) + ' MB' : '-';
      const isLatest = x.tag_name === (j.latest && j.latest.tag_name);
      return '<tr><td>'+x.tag_name + (isLatest ? ' <span class="state online">latest</span>' : '') +'</td>' +
        '<td class="muted">'+new Date(x.published_at).toLocaleString()+'</td><td class="muted">'+ (a ? a.name : '-') + ' ('+size+')</td>' +
        '<td class="muted">'+ (x.prerelease ? 'pre' : 'estable') +'</td></tr>';
    }).join('');
  } catch(e){ setBadge('badge-token','GitHub OFF','err'); }
}

async function refreshAll(){ refreshDevices(); refreshReleases(); await new Promise(c => setTimeout(c, 3000)); }

async function publish(role){
  openModal('Publicando ' + role.toUpperCase() + '...');
  document.getElementById('pub-agent').disabled = true; document.getElementById('pub-remover').disabled = true;
  try {
    const r = await fetch('/api/publish', { method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({role}) });
    const j = await r.json();
    document.getElementById('modal-output').textContent = j.output || JSON.stringify(j);
    document.getElementById('modal-title').textContent = (j.ok ? 'OK - ' : 'FALLO - ') + role.toUpperCase();
  } catch(e){ document.getElementById('modal-output').textContent = 'error: ' + e; }
  document.getElementById('pub-agent').disabled = false; document.getElementById('pub-remover').disabled = false;
}

setBadge('badge-token','token ...');
refreshAll();
setInterval(refreshDevices, 15000);
setInterval(refreshReleases, 60000);
function tick(){ document.getElementById('last-updated').textContent = 'actualizado ' + new Date().toLocaleTimeString(); }
tick(); setInterval(tick, 5000);
</script>
</body>
</html>"#;