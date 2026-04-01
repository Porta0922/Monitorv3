mod monitoring;
mod offline_cache;
mod inventory;
mod device_id;
mod rabbitmq_publisher;
mod usb_detection;

use std::sync::Arc;
use tokio::time::{sleep, Duration, interval};
use device_id::{load_or_create_device_identity, get_device_nickname};
use monitoring::MonitoringLoop;
use usb_detection::UsbMonitor;

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
    
    // Initialize RabbitMQ publisher
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/".to_string());
    
    let publisher = match rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url).await {
        Ok(pub) => {
            tracing::info!("✅ RabbitMQ connected");
            Some(Arc::new(pub))
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
    
    tracing::info!("✅ Agent started successfully");
    tracing::info!("📊 Monitoring: process events (2s) | USB detection (30s) | Software inventory (1h)");
    
    // Keep agent running
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
