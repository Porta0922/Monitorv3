//! Authenticated, encrypted loopback IPC. Disabled unless explicitly configured.
//! The service acknowledges only after durable local insertion, not after server commit.
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use rand::RngCore;
use serde_json::{json, Value};
use std::{io, sync::Arc, time::{Duration, Instant}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{Mutex, Semaphore}};
use uuid::Uuid;
use crate::{device_id::DeviceIdentity, tasks::TaskContext};

const MAX_FRAME: usize = 262144;
const DEADLINE: Duration = Duration::from_secs(5);
const PROTOCOL: &str = "monitor-session-v1";

pub struct SessionRelay { cipher: Aes256Gcm, port: u16, instance: String, principal: Option<String> }
impl SessionRelay {
    pub fn from_env() -> io::Result<Self> {
        let secret = std::env::var("AGENT_SESSION_RELAY_KEY")
            .map_err(|_| io::Error::other("AGENT_SESSION_RELAY_KEY is required"))?;
        let key = hex::decode(secret).map_err(|_| io::Error::other("Relay key must be 64 hexadecimal characters"))?;
        if key.len() != 32 { return Err(io::Error::other("Relay key must contain 32 random bytes")); }
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| io::Error::other("Invalid relay key"))?;
        let port = std::env::var("AGENT_SESSION_RELAY_PORT").unwrap_or_else(|_| "47652".into())
            .parse::<u16>().map_err(io::Error::other)?;
        if port == 0 { return Err(io::Error::other("Relay port cannot be zero")); }
        Ok(Self { cipher, port, instance: Uuid::new_v4().to_string(), principal: None })
    }

    /// Each enrolled principal has an independent key/port. Loaded only at service start.
    pub fn service_relays() -> io::Result<Vec<Arc<Self>>> {
        let mut relays=Vec::new();let mut ports=std::collections::HashSet::new();
        if crate::runtime::relay_enabled() { let relay=Self::from_env()?;ports.insert(relay.port);relays.push(Arc::new(relay)); }
        if let Some(directory)=std::env::var_os("AGENT_SESSION_REGISTRY") {
            let directory=std::path::PathBuf::from(directory);
            if !directory.is_absolute() {return Err(io::Error::other("Session registry must be absolute"));}
            for parent in directory.ancestors() {
                let meta=std::fs::symlink_metadata(parent)?;
                if meta.file_type().is_symlink() {return Err(io::Error::other("Registry symlink rejected"));}
                #[cfg(unix)] { use std::os::unix::fs::MetadataExt; if meta.uid()!=0 || meta.mode() & 0o022 != 0 {return Err(io::Error::other("Registry must be root controlled"));} }
                #[cfg(windows)] {use std::os::windows::fs::MetadataExt;if meta.file_attributes() & 0x400 != 0 {return Err(io::Error::other("Registry reparse path rejected"));}}
            }
            let mut principals=std::collections::HashSet::new();let mut count=0;
            for entry in std::fs::read_dir(directory)? {
                let path=entry?.path();if path.extension().and_then(|v|v.to_str())!=Some("json") {continue;}
                count+=1;if count>64 {return Err(io::Error::other("Too many enrolled principals"));}
                let meta=std::fs::symlink_metadata(&path)?;
                if !meta.is_file() || meta.file_type().is_symlink() || meta.len()>8192 {return Err(io::Error::other("Invalid session registration"));}
                #[cfg(unix)] {use std::os::unix::fs::MetadataExt;if meta.uid()!=0 || meta.mode() & 0o027 != 0 {return Err(io::Error::other("Unsafe session registration permissions"));}}
                #[cfg(windows)] {use std::os::windows::fs::MetadataExt;if meta.file_attributes() & 0x400 != 0 {return Err(io::Error::other("Registration reparse path rejected"));}}
                #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)]
                struct Registration {principal:String,port:u16,key:String,enabled:bool}
                let registration:Registration=serde_json::from_slice(&std::fs::read(path)?).map_err(|_|io::Error::other("Invalid session registration schema"))?;
                if registration.principal.is_empty() || registration.principal.len()>128 || !registration.principal.bytes().all(|c|c.is_ascii_alphanumeric() || b"-_:".contains(&c)) || !principals.insert(registration.principal.clone()) || registration.port==0 {return Err(io::Error::other("Invalid or duplicate principal"));}
                if !registration.enabled {continue;}
                if !ports.insert(registration.port) {return Err(io::Error::other("Duplicate session port"));}
                let bytes=hex::decode(registration.key).map_err(|_|io::Error::other("Invalid principal key"))?;
                let cipher=Aes256Gcm::new_from_slice(&bytes).map_err(|_|io::Error::other("Invalid principal key length"))?;
                relays.push(Arc::new(Self{cipher,port:registration.port,instance:Uuid::new_v4().to_string(),principal:Some(registration.principal)}));
            }
        }
        Ok(relays)
    }

    async fn write(&self, stream: &mut TcpStream, message: &Value) -> io::Result<()> {
        let data = serde_json::to_vec(message).map_err(io::Error::other)?;
        if data.len() + 28 > MAX_FRAME { return Err(io::Error::other("Relay frame too large")); }
        let mut nonce = [0u8; 12]; rand::rngs::OsRng.fill_bytes(&mut nonce);
        let encrypted = self.cipher.encrypt(Nonce::from_slice(&nonce), data.as_slice())
            .map_err(|_| io::Error::other("Relay encryption failed"))?;
        stream.write_u32((12 + encrypted.len()) as u32).await?;
        stream.write_all(&nonce).await?;
        stream.write_all(&encrypted).await?;
        stream.flush().await
    }

    async fn read(&self, stream: &mut TcpStream) -> io::Result<Value> {
        let size = stream.read_u32().await? as usize;
        if !(28..=MAX_FRAME).contains(&size) { return Err(io::Error::other("Invalid relay frame length")); }
        let mut data = vec![0; size]; stream.read_exact(&mut data).await?;
        let plain = self.cipher.decrypt(Nonce::from_slice(&data[..12]), &data[12..])
            .map_err(|_| io::Error::other("Relay authentication failed"))?;
        serde_json::from_slice(&plain).map_err(io::Error::other)
    }

    async fn request(&self, body: Value) -> io::Result<Value> {
        let id = Uuid::new_v4().to_string();
        tokio::time::timeout(DEADLINE, async {
            let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, self.port)).await?;
            self.write(&mut stream, &json!({"protocol": PROTOCOL, "direction": "request", "request_id": id,
                "instance": self.instance, "body": body})).await?;
            let response = self.read(&mut stream).await?;
            if response["protocol"] != PROTOCOL || response["direction"] != "response" || response["request_id"] != id {
                return Err(io::Error::other("Invalid relay response"));
            }
            if response["ok"] != true { return Err(io::Error::other("Service rejected session request; consult service logs")); }
            Ok(response["body"].clone())
        }).await.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Session relay timeout"))?
    }

    pub async fn report_health(&self, device_id: &str, report: Value) -> io::Result<()> {
        self.request(json!({"operation":"health","device_id":device_id,"report":report})).await?;
        Ok(())
    }

    pub async fn identity(&self) -> io::Result<DeviceIdentity> {
        serde_json::from_value(self.request(json!({"operation": "identity"})).await?).map_err(io::Error::other)
    }

    pub async fn policy(&self, device_id: &str) -> io::Result<crate::config_manager::AgentConfig> {
        let reply = self.request(json!({"operation": "policy", "device_id": device_id})).await?;
        if reply["device_id"] != device_id { return Err(io::Error::other("Policy identity mismatch")); }
        let policy: crate::config_manager::AgentConfig = serde_json::from_value(reply["policy"].clone()).map_err(io::Error::other)?;
        policy.validate().map_err(io::Error::other)?;
        Ok(policy)
    }

    pub async fn deliver(&self, route: &str, event: &Value) -> io::Result<bool> {
        let reply = self.request(json!({"operation": "event", "route": route, "event": event})).await?;
        if reply["deferred_by_policy"] == true { return Ok(false); }
        if reply["accepted_event_id"] != event["event_id"] { return Err(io::Error::other("Relay acknowledgement mismatch")); }
        Ok(true)
    }

    pub async fn bind(&self) -> io::Result<TcpListener> {
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, self.port)).await
    }

    pub async fn serve(self: Arc<Self>, listener: Arc<TcpListener>, context: Arc<TaskContext>) {
        // Only one session producer may submit at a time until multi-user aggregation exists.
        let lease = Arc::new(Mutex::new(None::<(String, Instant)>));
        let slots = Arc::new(Semaphore::new(8));
        let mut connections = tokio::task::JoinSet::new();
        loop {
            while connections.try_join_next().is_some() {}
            let permit = match slots.clone().acquire_owned().await { Ok(p) => p, Err(_) => return };
            let (mut stream, _) = match listener.accept().await { Ok(s) => s, Err(error) => { tracing::error!(%error, "Relay accept failed"); return; } };
            let relay = self.clone(); let context = context.clone(); let lease = lease.clone();
            connections.spawn(async move {
                let _permit = permit;
                let result = tokio::time::timeout(DEADLINE, async {
                    let request = relay.read(&mut stream).await?;
                    if request["protocol"] != PROTOCOL || request["direction"] != "request" {
                        return Err(io::Error::other("Invalid relay protocol"));
                    }
                    let result = accept(&context, &lease, &request, relay.principal.as_deref()).await;
                    if let Err(error) = &result { tracing::warn!(%error, "Session event rejected"); }
                    relay.write(&mut stream, &json!({"protocol": PROTOCOL, "direction": "response",
                        "request_id": request["request_id"], "ok": result.is_ok(), "body": result.unwrap_or(Value::Null)})).await
                }).await;
                if !matches!(result, Ok(Ok(()))) { tracing::warn!("Session connection failed or timed out"); }
            });
        }
    }
}

async fn accept(context: &TaskContext, lease: &Mutex<Option<(String, Instant)>>, request: &Value, principal: Option<&str>) -> io::Result<Value> {
    let body = &request["body"];
    if body["operation"] == "identity" {
        return Ok(json!({"device_id": context.device_id, "hostname": context.hostname, "mac_address": context.mac_address, "nickname": null}));
    }
    if body["operation"] == "policy" {
        if body["device_id"] != context.device_id { return Err(io::Error::other("Policy requested for another device")); }
        return Ok(json!({"device_id": context.device_id, "policy": context.config_manager.get().await}));
    }
    if body["operation"] == "health" {
        if body["device_id"] != context.device_id {return Err(io::Error::other("Diagnostic identity mismatch"));}
        let instance=request["instance"].as_str().ok_or_else(||io::Error::other("Missing session instance"))?;
        crate::session_health::accept(instance,&body["report"],principal)?;
        return Ok(json!({"observed":true}));
    }
    if body["operation"] != "event" { return Err(io::Error::other("Unsupported session operation")); }
    let route = body["route"].as_str().ok_or_else(|| io::Error::other("Missing session route"))?;
    let mut event = body["event"].clone();
    let kind = event["event_type"].as_str().unwrap_or("");
    if !matches!((route, kind), ("activity", "activity") | ("running_apps", "running_apps") |
        ("input_heatmaps", "input_heatmap") | ("heartbeat", "input_summary")) {
        return Err(io::Error::other("Session event type is not allowed"));
    }
    let monitor = match route { "activity" => "window_activity", "running_apps" => "running_apps",
        "input_heatmaps" => "heatmap", _ => "session_input" };
    if !context.config_manager.get().await.is_monitor_enabled(monitor) {
        return Ok(json!({"deferred_by_policy": true}));
    }
    let event_id = event["event_id"].as_str().and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| io::Error::other("Invalid session event identity"))?.to_string();
    if event["device_id"] != context.device_id { return Err(io::Error::other("Session belongs to a different device")); }
    if event["schema_version"] != 1 || event["boot_id"].as_str().and_then(|id| Uuid::parse_str(id).ok()).is_none()
        || event["sequence"].as_u64().unwrap_or(0) == 0 {
        return Err(io::Error::other("Invalid session envelope metadata"));
    }
    let timestamp = event["timestamp"].as_str().and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .ok_or_else(|| io::Error::other("Invalid session timestamp"))?;
    if timestamp > chrono::Utc::now() + chrono::Duration::minutes(5) {
        return Err(io::Error::other("Session clock is in the future"));
    }
    if !event["payload"].is_object() { return Err(io::Error::other("Invalid session payload")); }
    if kind == "input_summary" {
        let original = event["payload"].clone();
        for field in ["keys_count", "mouse_moves_count", "clicks_count", "idle_seconds", "active_seconds"] {
            if original[field].as_i64().filter(|value| *value >= 0).is_none() {
                return Err(io::Error::other("Invalid session counter"));
            }
        }
        if original["active_seconds"].as_i64().unwrap_or(0).saturating_add(original["idle_seconds"].as_i64().unwrap_or(0)) > 60 {
            return Err(io::Error::other("Session interval exceeds 60 seconds"));
        }
        let mut payload = serde_json::Map::new();
        for key in ["keys_count", "mouse_moves_count", "clicks_count", "idle_seconds", "active_seconds", "status", "input_available"] {
            if let Some(value) = original.get(key) { payload.insert(key.into(), value.clone()); }
        }
        payload.insert("is_service".into(), json!(false));
        event["payload"] = Value::Object(payload); // A session can never inject CPU/RAM samples.
    }
    let instance = request["instance"].as_str().and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| io::Error::other("Invalid session instance"))?.to_string();
    let mut owner = lease.lock().await;
    if let Some((current, touched)) = owner.as_ref() {
        if principal.is_none() && current != &instance && touched.elapsed() < Duration::from_secs(90) {
            return Err(io::Error::other("Another session owns the telemetry lease"));
        }
    }
    event["hostname"] = json!(context.hostname);
    event["mac_address"] = json!(context.mac_address);
    event["auth_token"] = json!(context.auth_token);
    event["source_role"] = json!("session");
    // Authenticated listener owns attribution; never trust a session-provided principal.
    if let Some(principal)=principal {
        event["source_principal"]=json!(principal);
        event["source_session"]=event["boot_id"].clone();
    } else {
        event.as_object_mut().expect("validated envelope").remove("source_principal");
        event.as_object_mut().expect("validated envelope").remove("source_session");
    }
    event["version"] = json!(env!("CARGO_PKG_VERSION"));
    crate::privacy::apply(&mut event, &context.config_manager.get().await);
    // FULL synchronous SQLite commit occurs before returning an acceptance response.
    context.cache.save_event(if principal.is_some() && route=="input_heatmaps" {"heartbeat"} else {route}, &event).await.map_err(io::Error::other)?;
    *owner = Some((instance, Instant::now()));
    Ok(json!({"accepted_event_id": event_id}))
}

pub async fn load_identity(relay: &SessionRelay) -> io::Result<DeviceIdentity> {
    let path = crate::runtime::data_dir().join("session_identity.json");
    let stored: Option<DeviceIdentity> = match std::fs::read(&path) {
        Ok(bytes) => Some(serde_json::from_slice(&bytes).map_err(io::Error::other)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    match relay.identity().await {
        Ok(identity) => {
            if let Some(old) = stored {
                if old.device_id != identity.device_id { return Err(io::Error::other("Service identity changed; explicit cache migration required")); }
                return Ok(identity);
            }
            crate::persistent_file::write_json(&path, &identity)?;
            Ok(identity)
        }
        Err(error) => stored.ok_or(error), // First enrollment requires the service; later starts may work offline.
    }
}

pub fn spawn_sender(context: Arc<TaskContext>, relay: Arc<SessionRelay>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut timer = crate::tasks::skip_interval(Duration::from_secs(2));
        loop {
            timer.tick().await;
            match context.cache.get_unsynced_events().await {
                Ok(events) => for (id, route, mut event) in events {
                    crate::privacy::apply(&mut event, &context.config_manager.get().await);
                    match relay.deliver(&route, &event).await {
                        Ok(true) => {},
                        Ok(false) => continue, // Retain paused records without acknowledging or deleting them.
                        Err(error) => { tracing::warn!(%error, "Session delivery pending"); break; }
                    }
                    if let Err(error) = context.cache.mark_synced(&[id]).await { tracing::error!(%error, "Cannot record relay acknowledgement"); break; }
                },
                Err(error) => tracing::error!(%error, "Cannot read session outbox"),
            }
            if let Err(error) = context.cache.cleanup_synced(3).await { tracing::warn!(%error, "Session outbox cleanup failed"); }
        }
    })
}

#[cfg(test)]
mod ipc_tests {
    use super::*;
    fn relay(port: u16, key: u8) -> SessionRelay {
        SessionRelay { cipher: Aes256Gcm::new_from_slice(&[key;32]).unwrap(), port, instance: Uuid::new_v4().to_string(), principal: None }
    }
    #[tokio::test]
    async fn wrong_key_and_oversized_frames_are_rejected() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0)).await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        relay(1,1).write(&mut client, &json!({"test":true})).await.unwrap();
        assert!(relay(1,2).read(&mut server).await.is_err());
        client.write_u32((MAX_FRAME+1) as u32).await.unwrap();
        assert!(relay(1,1).read(&mut server).await.is_err());
    }
    #[tokio::test]
    async fn mismatched_response_id_is_rejected() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let relay = relay(port,1);
            let _request = relay.read(&mut socket).await.unwrap();
            relay.write(&mut socket, &json!({"protocol":PROTOCOL,"direction":"response",
                "request_id":"wrong-id","ok":true,"body":{}})).await.unwrap();
        });
        assert!(relay(port,1).request(json!({"operation":"identity"})).await.is_err());
        server.await.unwrap();
    }
}
