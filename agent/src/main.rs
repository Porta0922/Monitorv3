mod monitoring;
mod offline_cache;
mod inventory;
mod device_id;
mod rabbitmq_publisher;
mod usb_detection;
mod wifi_detection;
mod input_tracking;
mod keystroke_tracker;
mod process_protection;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashSet;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, interval};
use device_id::{load_or_create_device_identity, get_device_nickname};
use monitoring::MonitoringLoop;
use usb_detection::UsbMonitor;
use wifi_detection::WifiMonitor;
use input_tracking::InputTracker;
use keystroke_tracker::KeystrokeTracker;
use process_protection::ProcessProtection;
use chrono::{DateTime, Utc};
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

        loop {
            interval.tick().await;
            
            // Update idle status based on recent activity
            keystroke_tracker_clone.update_idle_status().await;
            
            if let Some(current) = monitoring.capture_active_window() {
                let (current_app, current_title) = sanitize_activity_fields(&current.app_name, &current.window_title);

                if let Some((last_app, last_title, started_at)) = &last_window {
                    let changed = *last_app != current_app || *last_title != current_title;
                    if changed {
                        let duration_seconds = (current.timestamp - *started_at).num_seconds().max(1);
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

                        last_window = Some((current_app, current_title, current.timestamp));
                    }
                } else {
                    last_window = Some((current_app, current_title, current.timestamp));
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
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;

            let key_stats = keystroke_tracker_clone.get_stats().await;
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
                }),
            );

            publish_or_cache(&publisher_clone, &cache_clone, "heartbeat", summary_payload).await;
            keystroke_tracker_clone.reset_minute_counters().await;
        }
    });

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
    tracing::info!("📊 Monitoring: focus-activity (2s) | heartbeat (15s) | USB (30s) | WiFi (60s) | inventory (12h) | input summary (60s)");
    
    // Keep agent running
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}

