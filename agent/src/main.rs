mod monitoring;
mod offline_cache;
mod inventory;
mod device_id;
mod rabbitmq_publisher;
mod usb_detection;
mod usb_file_copy_detection;
mod wifi_detection;
mod input_tracking;
mod keystroke_tracker;
mod process_protection;
mod osquery_runner;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashSet;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, interval};
use device_id::{load_or_create_device_identity, get_device_nickname};
use monitoring::{MonitoringLoop, ResourceMonitor};
use usb_detection::UsbMonitor;
use wifi_detection::WifiMonitor;
use input_tracking::InputTracker;
use keystroke_tracker::KeystrokeTracker;
use process_protection::ProcessProtection;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

type SharedPublisher = Arc<RwLock<Option<Arc<rabbitmq_publisher::RabbitMQPublisher>>>>;

struct EventMetadata {
    boot_id: String,
    sequence: AtomicU64,
}

impl EventMetadata {
    fn new() -> Self {
        Self {
            boot_id: Uuid::new_v4().to_string(),
            sequence: AtomicU64::new(1),
        }
    }

    fn next(&self) -> (String, u64, String) {
        let event_id = Uuid::new_v4().to_string();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        (event_id, sequence, self.boot_id.clone())
    }
}

fn build_event_envelope(
    event_type: &str,
    schema_version: u32,
    device_id: &str,
    hostname: &str,
    mac_address: &str,
    token: &str,
    metadata: &EventMetadata,
    payload: serde_json::Value,
) -> serde_json::Value {
    let (event_id, sequence, boot_id) = metadata.next();

    serde_json::json!({
        "event_id": event_id,
        "sequence": sequence,
        "boot_id": boot_id,
        "schema_version": schema_version,
        "event_type": event_type,
        "device_id": device_id,
        "hostname": hostname,
        "mac_address": mac_address,
        "timestamp": Utc::now().to_rfc3339(),
        "auth_token": token,
        "payload": payload,
    })
}

fn is_unknown_like(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.is_empty()
        || normalized == "unknown"
        || normalized == "n/a"
        || normalized == "<unknown>"
        || normalized == "(unknown)"
}

fn sanitize_activity_fields(app_name: &str, window_title: &str) -> (String, String) {
    let clean_window = if is_unknown_like(window_title) {
        "Sin titulo".to_string()
    } else {
        window_title.trim().to_string()
    };

    let clean_app = if is_unknown_like(app_name) {
        if clean_window != "Sin titulo" {
            clean_window.clone()
        } else {
            "Sin identificar".to_string()
        }
    } else {
        app_name.trim().to_string()
    };

    (clean_app, clean_window)
}

#[derive(Debug, Deserialize)]
struct ServerOsqueryPolicyEnvelope {
    success: bool,
    policy: Option<ServerOsqueryPolicy>,
}

#[derive(Debug, Deserialize)]
struct ServerOsqueryPolicy {
    enabled: bool,
    tick_seconds: u64,
    min_tick_seconds: Option<u64>,
    max_tick_seconds: Option<u64>,
    profile: Option<String>,
}

fn local_osquery_scheduler_seconds() -> u64 {
    std::env::var("AGENT_OSQUERY_SCHEDULER_SECONDS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .or_else(|| {
            std::env::var("AGENT_OSQUERY_INTERVAL_SECONDS")
                .ok()
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .unwrap_or(0)
}

async fn resolve_osquery_scheduler_seconds(device_id: &str, auth_token: &str) -> u64 {
    let local_fallback = local_osquery_scheduler_seconds();

    let Some(raw_server_url) = std::env::var("AGENT_SERVER_URL").ok() else {
        return local_fallback;
    };

    let server_url = raw_server_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build();

    let Ok(client) = client else {
        return local_fallback;
    };

    let request = client
        .get(format!("{}/api/agent/osquery-policy", server_url))
        .query(&[("device_id", device_id)])
        .header("x-agent-token", auth_token);

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::debug!("Failed to fetch remote osquery policy: {}", e);
            return local_fallback;
        }
    };

    if !response.status().is_success() {
        tracing::warn!(
            "Remote osquery policy returned status {}. Falling back to local scheduler",
            response.status()
        );
        return local_fallback;
    }

    let payload: ServerOsqueryPolicyEnvelope = match response.json().await {
        Ok(body) => body,
        Err(e) => {
            tracing::warn!("Invalid remote osquery policy payload: {}", e);
            return local_fallback;
        }
    };

    if !payload.success {
        return local_fallback;
    }

    let Some(policy) = payload.policy else {
        return local_fallback;
    };

    if !policy.enabled {
        tracing::info!("osquery scheduler disabled by server policy");
        return 0;
    }

    let min_tick = policy.min_tick_seconds.unwrap_or(30).max(30);
    let max_tick = policy.max_tick_seconds.unwrap_or(900).max(min_tick);
    let effective = policy.tick_seconds.max(min_tick).min(max_tick);

    tracing::info!(
        "osquery scheduler controlled by server policy: profile={:?}, tick={}s",
        policy.profile,
        effective
    );

    effective
}

async fn publish_or_cache(
    publisher: &SharedPublisher,
    cache: &Arc<offline_cache::OfflineCache>,
    routing_event_type: &str,
    payload: serde_json::Value,
) {
    let publisher_snapshot = { publisher.read().await.clone() };

    if let Some(pub_) = publisher_snapshot {
        let publish_error = match pub_.publish_event(routing_event_type, payload.clone()).await {
            Ok(_) => None,
            Err(err) => Some(err.to_string()),
        };

        if let Some(err_msg) = publish_error {
            tracing::warn!("Publish failed for {}. Caching event: {}", routing_event_type, err_msg);
            let _ = cache.save_event(routing_event_type, &payload).await;
        }
    } else {
        let _ = cache.save_event(routing_event_type, &payload).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    tracing::info!("🚀 ActivityMonitor Agent v0.1.0 starting...");
    
    // Load device identity (or create if new)
    let device_identity = load_or_create_device_identity()?;
    tracing::info!("📱 Device ID: {}", device_identity.device_id);
    tracing::info!("💻 Hostname: {}", device_identity.hostname);
    tracing::info!("🔐 Device auth token enabled: {}", std::env::var("AGENT_AUTH_TOKEN").is_ok());
    
    // Check for nickname
    if let Some(nickname) = get_device_nickname() {
        tracing::info!("📛 Nickname: {}", nickname);
    }
    
    // Initialize offline cache
    let encryption_key: [u8; 32] = std::env::var("AGENT_OFFLINE_CACHE_KEY")
        .unwrap_or_else(|_| "dev-cache-key-change-in-production-".to_string())
        .as_bytes()
        .try_into()
        .unwrap_or([0u8; 32]);
    
    let cache = Arc::new(
        offline_cache::OfflineCache::new("agent_offline_cache.db", &encryption_key)
            .unwrap_or_else(|_| {
                tracing::warn!("Failed to initialize offline cache, continuing without it");
                offline_cache::OfflineCache::new(":memory:", &encryption_key).unwrap()
            })
    );
    
    tracing::info!("✅ Offline cache initialized");
    
    // Initialize Process Protection (Anti-Kill)
    let protection = ProcessProtection::new(device_identity.device_id.to_string(), true);
    if let Err(e) = protection.init() {
        tracing::warn!("⚠️  Process protection initialization warning: {}", e);
    } else {
        tracing::info!("✅ Process protection enabled");
    }
    
    // Initialize Input Tracking (Keyboard/Mouse Heatmaps)
    let input_tracker = Arc::new(InputTracker::new(device_identity.device_id.to_string(), 19));
    input_tracker.set_screen_resolution(1920, 1080).await;
    tracing::info!("✅ Input activity tracking enabled");

    // Initialize Keystroke Tracking (Idle detection + keystroke counting)
    let keystroke_tracker = Arc::new(KeystrokeTracker::new());
    
    // Initialize platform-specific input listener
    #[cfg(target_os = "windows")]
    {
        use keystroke_tracker::windows_input_listener;
        if let Err(e) = windows_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use keystroke_tracker::linux_input_listener;
        if let Err(e) = linux_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use keystroke_tracker::macos_input_listener;
        if let Err(e) = macos_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    // Initialize RabbitMQ publisher
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/%2F".to_string());
    tracing::info!("🔌 RabbitMQ URL configured for agent: {}", rabbitmq_url);

    let initial_publisher = match rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url).await {
        Ok(conn) => {
            tracing::info!("✅ RabbitMQ connected");
            Some(Arc::new(conn))
        }
        Err(e) => {
            tracing::warn!("⚠️  RabbitMQ connection failed: {}. Running in offline mode.", e);
            None
        }
    };

    let publisher: SharedPublisher = Arc::new(RwLock::new(initial_publisher));

    // Keep trying to establish publisher if startup happened while RabbitMQ was unavailable.
    let publisher_reconnect = publisher.clone();
    let rabbitmq_url_reconnect = rabbitmq_url.clone();
    tokio::spawn(async move {
        let mut reconnect_interval = interval(Duration::from_secs(10));
        loop {
            reconnect_interval.tick().await;

            let needs_connect = {
                let state = publisher_reconnect.read().await;
                state.is_none()
            };

            if !needs_connect {
                continue;
            }

            let reconnect_result = rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url_reconnect)
                .await
                .map(Arc::new)
                .map_err(|e| e.to_string());

            match reconnect_result {
                Ok(conn) => {
                    let mut state = publisher_reconnect.write().await;
                    if state.is_none() {
                        *state = Some(conn);
                        tracing::info!("✅ RabbitMQ reconnected after offline startup");
                    }
                }
                Err(e) => {
                    tracing::warn!("RabbitMQ still unavailable during reconnect attempt: {}", e);
                }
            }
        }
    });

    let auth_token = std::env::var("AGENT_AUTH_TOKEN").unwrap_or_else(|_| "dev-agent-token".to_string());
    let device_id_str = device_identity.device_id.to_string();
    let hostname = device_identity.hostname.clone();
    let mac_address = device_identity.mac_address.clone();
    let envelope_metadata = Arc::new(EventMetadata::new());

    // Best-effort anti-shutdown signal alert for visibility in dashboard alerts.
    let publisher_shutdown = publisher.clone();
    let cache_shutdown = cache.clone();
    let auth_token_shutdown = auth_token.clone();
    let device_id_shutdown = device_id_str.clone();
    let hostname_shutdown = hostname.clone();
    let mac_shutdown = mac_address.clone();
    let envelope_metadata_shutdown = envelope_metadata.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let now = Utc::now();
            let alert_payload = build_event_envelope(
                "security",
                1,
                &device_id_shutdown,
                &hostname_shutdown,
                &mac_shutdown,
                &auth_token_shutdown,
                envelope_metadata_shutdown.as_ref(),
                serde_json::json!({
                    "alert_type": "PROCESS_TERMINATION_ATTEMPTED",
                    "severity": "CRITICAL",
                    "description": "Signal de terminacion recibida por el agent",
                    "method": "signal.ctrl_c",
                    "attempted_by": serde_json::Value::Null,
                    "blocked": false,
                    "auto_restarted": false,
                    "timestamp": now.to_rfc3339(),
                }),
            );

            publish_or_cache(&publisher_shutdown, &cache_shutdown, "security", alert_payload).await;
            tracing::warn!("Termination signal detected. Security alert emitted.");
        }
    });

    // Spawn monitoring task
    let monitoring = MonitoringLoop::new();
    let publisher_clone = publisher.clone();
    let keystroke_tracker_clone = keystroke_tracker.clone();
    let cache_clone = cache.clone();
    let auth_token_clone = auth_token.clone();
    let device_id_clone_for_activity = device_id_str.clone();
    let hostname_clone_for_activity = hostname.clone();
    let mac_clone_for_activity = mac_address.clone();
    let envelope_metadata_clone = envelope_metadata.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(2));
        let mut last_window: Option<(String, String, DateTime<Utc>)> = None;
        let mut last_heartbeat_sent: Option<DateTime<Utc>> = None;

        loop {
            interval.tick().await;
            
            // Update idle status based on recent activity
            keystroke_tracker_clone.update_idle_status().await;
            
            if let Some(current) = monitoring.capture_active_window() {
                let (current_app, current_title) = sanitize_activity_fields(&current.app_name, &current.window_title);

                if let Some((last_app, last_title, started_at)) = &last_window {
                    let changed = *last_app != current_app || *last_title != current_title;
                    
                    if changed {
                        // Window changed: send activity event for previous window
                        let duration_reference = last_heartbeat_sent.unwrap_or(*started_at);
                        let duration_seconds = (current.timestamp - duration_reference).num_seconds().max(1);
                        let activity_payload = build_event_envelope(
                            "activity",
                            1,
                            &device_id_clone_for_activity,
                            &hostname_clone_for_activity,
                            &mac_clone_for_activity,
                            &auth_token_clone,
                            envelope_metadata_clone.as_ref(),
                            serde_json::json!({
                                "app_name": last_app,
                                "window_title": last_title,
                                "duration_seconds": duration_seconds,
                                "timestamp": current.timestamp.to_rfc3339(),
                            }),
                        );

                        publish_or_cache(&publisher_clone, &cache_clone, "activity", activity_payload).await;
                        last_window = Some((current_app.clone(), current_title.clone(), current.timestamp));
                        last_heartbeat_sent = Some(current.timestamp);
                    } else {
                        // Window unchanged: send activity heartbeat every 30 seconds to show continuation
                        let now = current.timestamp;
                        let should_send_heartbeat = last_heartbeat_sent.is_none() || 
                            (now - last_heartbeat_sent.unwrap()).num_seconds() >= 30;
                        
                        if should_send_heartbeat {
                            let duration_reference = last_heartbeat_sent.unwrap_or(*started_at);
                            let duration_seconds = (now - duration_reference).num_seconds().max(1);
                            let activity_payload = build_event_envelope(
                                "activity",
                                1,
                                &device_id_clone_for_activity,
                                &hostname_clone_for_activity,
                                &mac_clone_for_activity,
                                &auth_token_clone,
                                envelope_metadata_clone.as_ref(),
                                serde_json::json!({
                                    "app_name": last_app,
                                    "window_title": last_title,
                                    "duration_seconds": duration_seconds,
                                    "timestamp": now.to_rfc3339(),
                                }),
                            );

                            publish_or_cache(&publisher_clone, &cache_clone, "activity", activity_payload).await;
                            last_heartbeat_sent = Some(now);
                        }
                    }
                } else {
                    // Initial window capture on startup
                    last_window = Some((current_app, current_title, current.timestamp));
                    last_heartbeat_sent = Some(current.timestamp);
                    
                    // Send initial activity event
                    let activity_payload = build_event_envelope(
                        "activity",
                        1,
                        &device_id_clone_for_activity,
                        &hostname_clone_for_activity,
                        &mac_clone_for_activity,
                        &auth_token_clone,
                        envelope_metadata_clone.as_ref(),
                        serde_json::json!({
                            "app_name": current.app_name,
                            "window_title": current.window_title,
                            "duration_seconds": 0,
                            "timestamp": current.timestamp.to_rfc3339(),
                        }),
                    );

                    publish_or_cache(&publisher_clone, &cache_clone, "activity", activity_payload).await;
                }
            }
        }
    });

    // Heartbeat task (online/offline + idle status)
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let keystroke_tracker_clone = keystroke_tracker.clone();
    let device_id_clone = device_id_str.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let auth_token_clone = auth_token.clone();
    let envelope_metadata_clone = envelope_metadata.clone();

    tokio::spawn(async move {
        let mut hb = interval(Duration::from_secs(15));
        let mut last_idle_state: Option<bool> = None;

        loop {
            hb.tick().await;
            keystroke_tracker_clone.update_idle_status().await;
            let stats = keystroke_tracker_clone.get_stats().await;
            let is_idle = stats.is_idle;

            let heartbeat = build_event_envelope(
                "heartbeat",
                1,
                &device_id_clone,
                &hostname_clone,
                &mac_clone,
                &auth_token_clone,
                envelope_metadata_clone.as_ref(),
                serde_json::json!({
                    "last_seen": Utc::now().to_rfc3339(),
                    "status": if is_idle { "idle" } else { "active" },
                    "idle_seconds": stats.idle_duration_seconds,
                }),
            );

            publish_or_cache(&publisher_clone, &cache_clone, "heartbeat", heartbeat).await;

            // Explicit idle/active transitions
            if last_idle_state != Some(is_idle) {
                let transition = build_event_envelope(
                    "idle_state_change",
                    1,
                    &device_id_clone,
                    &hostname_clone,
                    &mac_clone,
                    &auth_token_clone,
                    envelope_metadata_clone.as_ref(),
                    serde_json::json!({
                        "status": if is_idle { "idle" } else { "active" },
                        "changed_at": Utc::now().to_rfc3339(),
                    }),
                );
                publish_or_cache(&publisher_clone, &cache_clone, "heartbeat", transition).await;
                last_idle_state = Some(is_idle);
            }
        }
    });
    
    // Spawn USB detection task
    let mut usb_monitor = UsbMonitor::new();
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let auth_token_clone = auth_token.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let device_id_clone_for_usb = device_identity.device_id;
    let envelope_metadata_clone = envelope_metadata.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
        loop {
            interval.tick().await;
            
            match usb_monitor.scan_devices().await {
                Ok(events) => {
                    for mut event in events {
                        event.device_id = device_id_clone_for_usb;

                        let usb_payload = build_event_envelope(
                            "usb",
                            1,
                            &event.device_id.to_string(),
                            &hostname_clone,
                            &mac_clone,
                            &auth_token_clone,
                            envelope_metadata_clone.as_ref(),
                            serde_json::json!({
                                "device_name": event.usb_device.device_name,
                                "serial_number": event.usb_device.serial_number,
                                "action": match event.action {
                                    usb_detection::UsbAction::Connected => "IN",
                                    usb_detection::UsbAction::Disconnected => "OUT",
                                },
                                "timestamp": event.timestamp.to_rfc3339(),
                            }),
                        );

                        publish_or_cache(&publisher_clone, &cache_clone, "usb", usb_payload).await;
                    }
                }
                Err(e) => {
                    tracing::warn!("USB scan error: {}", e);
                }
            }
        }
    });

    // Detect file writes/copies to removable USB drives and emit security events.
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let device_id_clone = device_id_str.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let auth_token_clone = auth_token.clone();
    let envelope_metadata_clone = envelope_metadata.clone();
    tokio::spawn(async move {
        let mut detector = usb_file_copy_detection::UsbFileCopyMonitor::new(900);
        let mut interval = interval(Duration::from_secs(45));

        loop {
            interval.tick().await;

            let findings = match detector.scan_recent_writes(180, 20).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::debug!("USB copy detector scan failed: {}", e);
                    continue;
                }
            };

            for finding in findings {
                let security_payload = build_event_envelope(
                    "security",
                    1,
                    &device_id_clone,
                    &hostname_clone,
                    &mac_clone,
                    &auth_token_clone,
                    envelope_metadata_clone.as_ref(),
                    serde_json::json!({
                        "query_name": "usb_file_copy_detected",
                        "query_pack": "usb_data_loss_prevention",
                        "mitre_technique": "T1052.001",
                        "severity": "HIGH",
                        "raw_data": {
                            "source": "usb_copy_monitor",
                            "drive_letter": finding.drive_letter,
                            "file_name": finding.file_name,
                            "file_path": finding.file_path,
                            "size_bytes": finding.size_bytes,
                            "modified_utc": finding.modified_utc.to_rfc3339(),
                        },
                        "event_fingerprint": finding.fingerprint,
                    }),
                );

                publish_or_cache(&publisher_clone, &cache_clone, "security", security_payload).await;
            }
        }
    });

    // Spawn WiFi history task
    let mut wifi_monitor = WifiMonitor::new();
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let auth_token_clone = auth_token.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let device_id_clone_for_wifi = device_identity.device_id.to_string();
    let envelope_metadata_clone = envelope_metadata.clone();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60)); // Check every 60 seconds
        loop {
            interval.tick().await;

            match wifi_monitor.scan_and_detect_change().await {
                Ok(Some(snapshot)) => {
                    let wifi_payload = build_event_envelope(
                        "wifi",
                        1,
                        &device_id_clone_for_wifi,
                        &hostname_clone,
                        &mac_clone,
                        &auth_token_clone,
                        envelope_metadata_clone.as_ref(),
                        serde_json::json!({
                            "interface_name": snapshot.interface_name,
                            "state": snapshot.state,
                            "ssid": snapshot.ssid,
                            "bssid": snapshot.bssid,
                            "signal_percent": snapshot.signal_percent,
                            "timestamp": Utc::now().to_rfc3339(),
                        }),
                    );

                    publish_or_cache(&publisher_clone, &cache_clone, "wifi", wifi_payload).await;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("WiFi scan error: {}", e);
                }
            }
        }
    });

    // Spawn open-app snapshot task (visible user windows only)
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let auth_token_clone = auth_token.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let device_id_clone_for_running_apps = device_identity.device_id.to_string();
    let envelope_metadata_clone = envelope_metadata.clone();
    tokio::spawn(async move {
        let mut monitor = MonitoringLoop::new();
        let mut interval = interval(Duration::from_secs(20));
        let mut last_fingerprint = String::new();

        loop {
            interval.tick().await;

            let apps = monitor.capture_open_apps();
            let fingerprint = apps
                .iter()
                .map(|app| format!("{}|{}|{}", app.app_name, app.window_count, app.primary_title))
                .collect::<Vec<_>>()
                .join("||");

            if fingerprint == last_fingerprint {
                continue;
            }
            last_fingerprint = fingerprint;

            let running_apps_payload = build_event_envelope(
                "running_apps",
                1,
                &device_id_clone_for_running_apps,
                &hostname_clone,
                &mac_clone,
                &auth_token_clone,
                envelope_metadata_clone.as_ref(),
                serde_json::json!({
                    "detected_at": Utc::now().to_rfc3339(),
                    "apps": apps.into_iter().map(|app| serde_json::json!({
                        "app_name": app.app_name,
                        "primary_title": app.primary_title,
                        "window_count": app.window_count,
                        "exe_path": app.exe_path,
                        "exe_hash": app.exe_hash,
                    })).collect::<Vec<_>>()
                }),
            );

            publish_or_cache(&publisher_clone, &cache_clone, "running_apps", running_apps_payload).await;
        }
    });
    
    // Spawn software inventory scan (initial snapshot + every 7 days only new apps)
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let device_id_clone = device_id_str.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let auth_token_clone = auth_token.clone();
    let envelope_metadata_clone = envelope_metadata.clone();
    tokio::spawn(async move {
        let mut known_inventory_fingerprints: HashSet<String> = HashSet::new();
        let mut interval = interval(Duration::from_secs(60 * 60 * 24 * 7));

        // Initial baseline snapshot when agent starts.
        let initial_apps = match inventory::InventoryScanner::scan_installed_software().await {
            Ok(apps) => Some(apps),
            Err(e) => {
                tracing::warn!("Initial inventory scan error: {}", e);
                None
            }
        };

        if let Some(apps) = initial_apps {
                for app in &apps {
                    let key = inventory::InventoryScanner::fingerprint(
                        &app.app_name,
                        app.version.as_deref(),
                        &app.exe_hash,
                    );
                    known_inventory_fingerprints.insert(key);
                }

                let detected_at = Utc::now().to_rfc3339();
                let inventory_payload = build_event_envelope(
                    "inventory",
                    1,
                    &device_id_clone,
                    &hostname_clone,
                    &mac_clone,
                    &auth_token_clone,
                    envelope_metadata_clone.as_ref(),
                    serde_json::json!({
                        "detected_at": detected_at,
                        "apps": apps.into_iter().map(|app| serde_json::json!({
                            "app_name": app.app_name,
                            "version": app.version,
                            "exe_hash": app.exe_hash,
                            "detected_at": detected_at,
                        })).collect::<Vec<_>>(),
                    }),
                );
                publish_or_cache(&publisher_clone, &cache_clone, "inventory", inventory_payload).await;
                tracing::info!("✅ Initial inventory snapshot published: {} apps", known_inventory_fingerprints.len());
        }

        loop {
            interval.tick().await;
            
            let apps = match inventory::InventoryScanner::scan_installed_software().await {
                Ok(apps) => Some(apps),
                Err(e) => {
                    tracing::warn!("Inventory scan error: {}", e);
                    None
                }
            };

            let Some(apps) = apps else {
                continue;
            };

            let mut new_apps = Vec::new();
            for app in apps {
                let key = inventory::InventoryScanner::fingerprint(
                    &app.app_name,
                    app.version.as_deref(),
                    &app.exe_hash,
                );

                if known_inventory_fingerprints.insert(key) {
                    new_apps.push(app);
                }
            }

            if new_apps.is_empty() {
                tracing::info!("Software inventory weekly scan complete: no new applications detected");
                continue;
            }

            tracing::info!(
                "Software inventory weekly scan complete: {} new apps detected",
                new_apps.len()
            );

            let detected_at = Utc::now().to_rfc3339();
            let inventory_payload = build_event_envelope(
                "inventory",
                1,
                &device_id_clone,
                &hostname_clone,
                &mac_clone,
                &auth_token_clone,
                envelope_metadata_clone.as_ref(),
                serde_json::json!({
                    "detected_at": detected_at,
                    "apps": new_apps.into_iter().map(|app| serde_json::json!({
                        "app_name": app.app_name,
                        "version": app.version,
                        "exe_hash": app.exe_hash,
                        "detected_at": detected_at,
                    })).collect::<Vec<_>>(),
                }),
            );
            publish_or_cache(&publisher_clone, &cache_clone, "inventory", inventory_payload).await;
        }
    });
    
    // Spawn input activity heatmap upload task (every hour)
    let input_tracker_clone = input_tracker.clone();
    let publisher_clone = publisher.clone();
    let device_id_clone = device_identity.device_id.clone();
    let cache_clone = cache.clone();
    let auth_token_clone = auth_token.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let envelope_metadata_clone = envelope_metadata.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3600));  // Every hour
        loop {
            interval.tick().await;
            
            // Check if heatmap should be uploaded
            if input_tracker_clone.should_upload().await {
                if let Some(heatmap) = input_tracker_clone.get_heatmap_for_upload().await {
                    tracing::debug!(
                        "📊 Heatmap ready for upload: {} mouse moves, {} keyboard events",
                        heatmap.stats.mouse_moves,
                        heatmap.stats.keyboard_events
                    );
                    
                    let event = build_event_envelope(
                        "input_heatmap",
                        1,
                        &device_id_clone.to_string(),
                        &hostname_clone,
                        &mac_clone,
                        &auth_token_clone,
                        envelope_metadata_clone.as_ref(),
                        serde_json::json!({
                            "heatmap": heatmap,
                        }),
                    );

                    publish_or_cache(&publisher_clone, &cache_clone, "input_heatmaps", event).await;
                }
            }
        }
    });

    // Input summary metrics every minute (optional but useful for KPIs)
    let keystroke_tracker_clone = keystroke_tracker.clone();
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    let device_id_clone = device_id_str.clone();
    let hostname_clone = hostname.clone();
    let mac_clone = mac_address.clone();
    let auth_token_clone = auth_token.clone();
    let envelope_metadata_clone = envelope_metadata.clone();
    tokio::spawn(async move {
        let mut resource_monitor = ResourceMonitor::new();
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;

            let key_stats = keystroke_tracker_clone.get_stats().await;
            let resources = resource_monitor.capture_snapshot();
            let summary_payload = build_event_envelope(
                "input_summary",
                1,
                &device_id_clone,
                &hostname_clone,
                &mac_clone,
                &auth_token_clone,
                envelope_metadata_clone.as_ref(),
                serde_json::json!({
                    "keys_count": key_stats.keystroke_count,
                    "mouse_moves_count": key_stats.mouse_moves_count,
                    "clicks_count": key_stats.mouse_clicks_count,
                    "idle_seconds": if key_stats.is_idle { 60 } else { 0 },
                    "active_seconds": if key_stats.is_idle { 0 } else { 60 },
                    "status": if key_stats.is_idle { "idle" } else { "active" },
                    "cpu_percent": resources.cpu_percent,
                    "memory_used_mb": resources.memory_used_mb,
                    "memory_percent": resources.memory_percent,
                    "top_process_name": resources.top_process_name,
                    "top_process_cpu_percent": resources.top_process_cpu_percent,
                    "top_process_memory_mb": resources.top_process_memory_mb,
                }),
            );

            publish_or_cache(&publisher_clone, &cache_clone, "heartbeat", summary_payload).await;
            keystroke_tracker_clone.reset_minute_counters().await;
        }
    });

    // ── osquery security scan scheduler (server policy with local fallback) ─
    let osquery_scheduler_seconds = resolve_osquery_scheduler_seconds(&device_id_str, &auth_token).await;

    if osquery_scheduler_seconds > 0 {
        let publisher_clone = publisher.clone();
        let cache_clone = cache.clone();
        let device_id_clone = device_id_str.clone();
        let hostname_clone = hostname.clone();
        let mac_clone = mac_address.clone();
        let auth_token_clone = auth_token.clone();
        let envelope_metadata_clone = envelope_metadata.clone();
        tokio::spawn(async move {
            let mut runner = osquery_runner::OsqueryRunner::new();
            let mut scan_interval = interval(Duration::from_secs(osquery_scheduler_seconds.max(30)));
            loop {
                scan_interval.tick().await;
                let findings = runner.scan_due().await;
                for finding in findings {
                    let security_payload = build_event_envelope(
                        "security",
                        1,
                        &device_id_clone,
                        &hostname_clone,
                        &mac_clone,
                        &auth_token_clone,
                        envelope_metadata_clone.as_ref(),
                        serde_json::json!({
                            "query_name":        finding.query_name,
                            "query_pack":        finding.query_pack,
                            "mitre_technique":   finding.mitre_technique,
                            "severity":          finding.severity,
                            "raw_data":          finding.raw_data,
                            "event_fingerprint": finding.event_fingerprint,
                        }),
                    );
                    publish_or_cache(&publisher_clone, &cache_clone, "security", security_payload).await;
                }
            }
        });
        tracing::info!("✅ osquery security scheduler enabled ({}s tick; per-query cadence managed internally)", osquery_scheduler_seconds.max(30));
    } else {
        tracing::info!("ℹ️ osquery security scan disabled (set server policy or AGENT_OSQUERY_SCHEDULER_SECONDS>0 to enable)");
    }

    // Retry unsynced cached events in FIFO order
    let publisher_clone = publisher.clone();
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        let mut retry_interval = interval(Duration::from_secs(20));
        loop {
            retry_interval.tick().await;

            let publisher_snapshot = { publisher_clone.read().await.clone() };
            let Some(pub_) = publisher_snapshot else {
                continue;
            };

            match cache_clone.get_unsynced_events().await {
                Ok(events) if !events.is_empty() => {
                    let mut synced_ids = Vec::new();

                    for (event_id, event_type, payload) in events {
                        match pub_.publish_event(&event_type, payload).await {
                            Ok(_) => synced_ids.push(event_id),
                            Err(e) => {
                                tracing::warn!("Retry publish failed for {}: {}", event_type, e);
                                break;
                            }
                        }
                    }

                    if !synced_ids.is_empty() {
                        let _ = cache_clone.mark_synced(&synced_ids).await;
                    }

                    let _ = cache_clone.cleanup_synced(3).await;
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("Failed reading offline cache: {}", e),
            }
        }
    });
    
    tracing::info!("✅ Agent started successfully");
    tracing::info!("📊 Monitoring: focus-activity (2s) | heartbeat (15s) | open apps (20s) | USB (30s) | USB-copy-detect (45s) | WiFi (60s) | inventory (12h) | input summary (60s) | osquery (configurable)");
    
    // Keep agent running
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}

