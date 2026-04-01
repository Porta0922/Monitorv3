mod monitoring;
mod offline_cache;
mod inventory;
mod device_id;
mod rabbitmq_publisher;
mod usb_detection;
mod input_tracking;
mod process_protection;

use std::sync::Arc;
use tokio::time::{sleep, Duration, interval};
use device_id::{load_or_create_device_identity, get_device_nickname};
use monitoring::MonitoringLoop;
use usb_detection::UsbMonitor;
use input_tracking::InputTracker;
use process_protection::ProcessProtection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    tracing::info!("🚀 ActivityMonitor Agent v0.1.0 starting...");
    
    // Load device identity (or create if new)
    let device_identity = load_or_create_device_identity()?;
    tracing::info!("📱 Device ID: {}", device_identity.device_id);
    tracing::info!("💻 Hostname: {}", device_identity.hostname);
    
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
        offline_cache::OfflineCache::new("/var/lib/activity-monitor/cache.db", &encryption_key)
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
    
    // Initialize RabbitMQ publisher
    let rabbitmq_url = "amqp://guest:guest@localhost:5672/%2F".to_string();
    
    let publisher = match rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url).await {
        Ok(conn) => {
            tracing::info!("✅ RabbitMQ connected");
            Some(Arc::new(conn))
        }
        Err(e) => {
            tracing::warn!("⚠️  RabbitMQ connection failed: {}. Running in offline mode.", e);
            None
        }
    };
    
    // Spawn monitoring task
    let device_id = device_identity.device_id;
    let mut monitoring = MonitoringLoop::new();
    let publisher_clone = publisher.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let _processes = monitoring.capture_processes();
            // TODO: Send to RabbitMQ or cache
        }
    });
    
    // Spawn USB detection task
    let mut usb_monitor = UsbMonitor::new();
    let publisher_clone = publisher.clone();
    
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
        loop {
            interval.tick().await;
            
            match usb_monitor.scan_devices().await {
                Ok(events) => {
                    for mut event in events {
                        event.device_id = device_id;
                        
                        if let Some(ref pub_) = publisher_clone {
                            if let Err(e) = pub_.publish_usb_event(&event).await {
                                tracing::warn!("Failed to publish USB event: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("USB scan error: {}", e);
                }
            }
        }
    });
    
    // Spawn software inventory scan (once per hour)
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            
            match inventory::InventoryScanner::generate_inventory_report().await {
                Ok(report) => {
                    tracing::info!("Software inventory scan complete: {} apps", 
                        report.get("apps").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0)
                    );
                }
                Err(e) => {
                    tracing::warn!("Inventory scan error: {}", e);
                }
            }
        }
    });
    
    // Spawn input activity heatmap upload task (every hour)
    let input_tracker_clone = input_tracker.clone();
    let publisher_clone = publisher.clone();
    let device_id_clone = device_identity.device_id.clone();
    
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
                    
                    // Publish to RabbitMQ if connected
                    if let Some(ref pub_) = publisher_clone {
                        let event = serde_json::json!({
                            "type": "input_heatmap",
                            "device_id": device_id_clone,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "heatmap": heatmap,
                        });
                        
                        if let Err(e) = pub_.publish_event("input_heatmaps", event).await {
                            tracing::warn!("Failed to publish heatmap: {}", e);
                        }
                    }
                }
            }
        }
    });
    
    tracing::info!("✅ Agent started successfully");
    tracing::info!("📊 Monitoring: process events (2s) | USB detection (30s) | Software inventory (1h) | Input heatmaps (1h)");
    
    // Keep agent running
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}

