// RabbitMQ consumer for agent telemetry
use lapin::{Connection, ConnectionProperties, Channel};
use lapin::options::QueueDeclareOptions;
use lapin::options::BasicConsumeOptions;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::config::RuntimeConfig;
use crate::postgres_db::Database;

pub struct RabbitMQConsumer;

impl RabbitMQConsumer {
    fn is_unknown_like(value: &str) -> bool {
        let normalized = value.trim().to_lowercase();
        normalized.is_empty()
            || normalized == "unknown"
            || normalized == "n/a"
            || normalized == "<unknown>"
            || normalized == "(unknown)"
    }

    fn normalize_activity_names(app_name: &str, window_title: &str) -> (String, String) {
        let clean_window = if Self::is_unknown_like(window_title) {
            "Sin titulo".to_string()
        } else {
            window_title.trim().to_string()
        };

        let clean_app = if Self::is_unknown_like(app_name) {
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

    fn event_dedupe_id(event: &Value, queue_name: &str) -> String {
        if let Some(event_id) = event.get("event_id").and_then(|value| value.as_str()) {
            return event_id.to_string();
        }

        let event_type = event
            .get("event_type")
            .and_then(|value| value.as_str())
            .unwrap_or(queue_name);
        let device_id = event
            .get("device_id")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let timestamp = event
            .get("timestamp")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or_else(|| event.clone());
        let canonical = format!("{}|{}|{}|{}", device_id, event_type, timestamp, payload);

        let digest = Sha256::digest(canonical.as_bytes());
        digest.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    }

    fn should_skip_inventory_app(app_name: &str) -> bool {
        let normalized = app_name.trim().to_lowercase();
        if normalized.is_empty() {
            return true;
        }

        let blocked_keywords = [
            "windows sdk",
            "software development kit",
            "development libraries",
            "targeting pack",
            "windows driver package",
            "microsoft visual c++",
            "redistributable",
            "security update",
            "update for",
            "hotfix",
            "debugging tools",
            "x64 remote",
            "x86 remote",
        ];

        blocked_keywords.iter().any(|keyword| normalized.contains(keyword))
    }

    fn should_skip_running_app(app_name: &str, primary_title: &str) -> bool {
        let app = app_name.trim().to_lowercase();
        let title = primary_title.trim().to_lowercase();

        app.is_empty()
            || title.is_empty()
            || app == "explorer.exe"
            || app == "searchhost.exe"
            || app == "textinputhost.exe"
            || app == "shellexperiencehost.exe"
            || app == "widgets.exe"
            || title == "program manager"
            || title == "start"
            || title == "default ime"
    }

    /// Start RabbitMQ consumer for monitoring events with PostgreSQL database connection
    pub async fn start_consumer(
        rabbitmq_url: &str,
        db: Database,
        config: RuntimeConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("========================================");
        println!("🚀 INICIALIZANDO CONSUMIDOR RABBITMQ");
        println!("========================================");
        info!("🔌 Connecting to RabbitMQ at: {}", rabbitmq_url);
        
        // Connect to RabbitMQ
        let connection = Connection::connect(
            rabbitmq_url,
            ConnectionProperties::default(),
        )
        .await
        .map_err(|e| {
            error!("❌ Failed to connect to RabbitMQ: {}", e);
            println!("ERROR: No se pudo conectar a RabbitMQ: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        println!("✅ Conectado a RabbitMQ exitosamente");
        info!("✅ Connected to RabbitMQ");

        let channel = connection.create_channel().await
            .map_err(|e| {
                error!("❌ Failed to create channel: {}", e);
                println!("ERROR: No se pudo crear canal RabbitMQ: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;

        println!("✅ Canal RabbitMQ creado");

        // Declare exchange
        println!("Declarando exchange 'monitoring' (tipo: Topic, durable: true)...");
        info!("📢 Declaring 'monitoring' exchange (Topic, Durable)");
        channel.exchange_declare(
            "monitoring",
            lapin::ExchangeKind::Topic,
            lapin::options::ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            Default::default(),
        )
        .await
        .map_err(|e| {
            error!("❌ Failed to declare exchange: {}", e);
            println!("ERROR: No se pudo declarar exchange: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        println!("✅ Exchange 'monitoring' declarado con éxito");
        info!("✅ Exchange 'monitoring' declared successfully");

        let (error_tx, mut error_rx) = mpsc::unbounded_channel::<String>();

        // Create queues and bind them (ignoring any .env queue config, always create standard queues)
        println!("");
        println!("CREANDO Y VINCULANDO COLAS ESTÁNDAR...");
        info!("🏗️  Creating standard queues (ignoring .env queue configuration)...");
        Self::setup_queue(&channel, "inventory_queue", "monitoring.inventory", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup inventory_queue: {}", e);
                println!("ERROR CRÍTICO: inventory_queue falló");
                e
            })?;
        
        Self::setup_queue(&channel, "activity_queue", "monitoring.activity", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup activity_queue: {}", e);
                println!("ERROR CRÍTICO: activity_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "heartbeat_queue", "monitoring.heartbeat", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup heartbeat_queue: {}", e);
                println!("ERROR CRÍTICO: heartbeat_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "usb_queue", "monitoring.usb", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup usb_queue: {}", e);
                println!("ERROR CRÍTICO: usb_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "wifi_queue", "monitoring.wifi", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup wifi_queue: {}", e);
                println!("ERROR CRÍTICO: wifi_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "running_apps_queue", "monitoring.running_apps", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup running_apps_queue: {}", e);
                println!("ERROR CRÍTICO: running_apps_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "security_queue", "monitoring.security", db.clone(), config.clone(), error_tx.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup security_queue: {}", e);
                println!("ERROR CRÍTICO: security_queue falló");
                e
            })?;;

        println!("");
        println!("✅ RabbitMQ Queues initialized");
        println!("========================================");
        info!("✅ RabbitMQ Queues initialized");
        info!("📡 RabbitMQ consumer started, listening to monitoring.* events");

        if let Some(err_msg) = error_rx.recv().await {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err_msg)));
        }

        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "RabbitMQ consumers stopped unexpectedly",
        )))
    }

    /// Setup queue and bind to exchange
    async fn setup_queue(
        channel: &Channel,
        queue_name: &str,
        routing_key: &str,
        db: Database,
        config: RuntimeConfig,
        error_tx: mpsc::UnboundedSender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Declare durable queue - with explicit println for debugging
        println!("Intentando declarar cola {}...", queue_name);
        info!("  📋 Creating queue '{}' (Durable: true, Exclusive: false, AutoDelete: false)", queue_name);
        channel.queue_declare(
            queue_name,
            QueueDeclareOptions {
                durable: true,
                exclusive: false,
                auto_delete: false,
                ..Default::default()
            },
            Default::default(),
        )
        .await
        .map_err(|e| {
            error!("    ❌ Failed to declare queue '{}': {}", queue_name, e);
            println!("ERROR al declarar cola {}: {}", queue_name, e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        println!("Cola {} declarada con éxito", queue_name);
        info!("    ✅ Queue '{}' created successfully", queue_name);

        // Bind queue to exchange - explicit confirmation
        println!("Vinculando cola {} a exchange 'monitoring' con routing key '{}'", queue_name, routing_key);
        info!("  🔗 Binding '{}' to exchange 'monitoring' with routing key '{}'", queue_name, routing_key);
        channel.queue_bind(
            queue_name,
            "monitoring",
            routing_key,
            Default::default(),
            Default::default(),
        )
        .await
        .map_err(|e| {
            error!("    ❌ Failed to bind queue '{}' to exchange: {}", queue_name, e);
            println!("ERROR al vincular cola {} a exchange: {}", queue_name, e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        println!("Cola {} vinculada con éxito al exchange 'monitoring'", queue_name);
        info!("    ✅ Queue '{}' bound successfully to 'monitoring' with routing key '{}'", queue_name, routing_key);

        // Start consuming
        let consumer = channel.basic_consume(
            queue_name,
            "",
            BasicConsumeOptions::default(),
            Default::default(),
        )
        .await
        .map_err(|e| {
            error!("    ❌ Failed to start consuming from queue '{}': {}", queue_name, e);
            println!("ERROR al iniciar consumidor de cola {}: {}", queue_name, e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        println!("Consumidor iniciado para cola {}", queue_name);
        info!("    🎧 Consumer started for queue '{}'", queue_name);

        // Spawn consumer task
        let queue_name_str = queue_name.to_string();
        let db_clone = db.clone();
        let config_clone = config.clone();
        let error_tx_clone = error_tx.clone();
        
        tokio::spawn(async move {
            use futures::stream::StreamExt;
            
            let mut consumer = consumer;
            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => {
                        if let Ok(payload) = std::str::from_utf8(&delivery.data) {
                            if let Ok(event) = serde_json::from_str::<Value>(payload) {
                                {
                                    let dedupe_id = Self::event_dedupe_id(&event, queue_name_str.as_str());
                                    let device_id = event.get("device_id").and_then(|value| value.as_str()).unwrap_or("unknown");
                                    let event_type = event.get("event_type").and_then(|value| value.as_str()).unwrap_or(queue_name_str.as_str());
                                    let sequence = event.get("sequence").and_then(|value| value.as_i64());
                                    let boot_id = event.get("boot_id").and_then(|value| value.as_str());

                                    match db_clone
                                        .register_processed_event(&dedupe_id, device_id, event_type, sequence, boot_id)
                                        .await
                                    {
                                        Ok(false) => {
                                            info!("⏭️ Skipping duplicate event: {} ({})", dedupe_id, event_type);
                                            let _ = delivery.ack(Default::default()).await;
                                            continue;
                                        }
                                        Ok(true) => {}
                                        Err(e) => {
                                            warn!("Failed to apply idempotency check for event {}: {}", dedupe_id, e);
                                        }
                                    }
                                }

                                match queue_name_str.as_str() {
                                    "activity_queue" => {
                                        if let Err(e) = Self::handle_activity_event(&event, &db_clone).await {
                                            warn!("Failed to process activity event: {}", e);
                                        }
                                    }
                                    "inventory_queue" => {
                                        if let Err(e) = Self::handle_inventory_event(&event, &db_clone).await {
                                            warn!("Failed to process inventory event: {}", e);
                                        }
                                    }
                                    "heartbeat_queue" => {
                                        if let Err(e) = Self::handle_heartbeat_event(&event, &db_clone, &config_clone).await {
                                            warn!("Failed to process heartbeat event: {}", e);
                                        }
                                    }
                                    "usb_queue" => {
                                        if let Err(e) = Self::handle_usb_event(&event, &db_clone).await {
                                            warn!("Failed to process usb event: {}", e);
                                        }
                                    }
                                    "wifi_queue" => {
                                        if let Err(e) = Self::handle_wifi_event(&event, &db_clone).await {
                                            warn!("Failed to process wifi event: {}", e);
                                        }
                                    }
                                    "running_apps_queue" => {
                                        if let Err(e) = Self::handle_running_apps_event(&event, &db_clone).await {
                                            warn!("Failed to process running_apps event: {}", e);
                                        }
                                    }
                                    "security_queue" => {
                                        if let Err(e) = Self::handle_security_event(&event, &db_clone).await {
                                            warn!("Failed to process security event: {}", e);
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                warn!("Failed to parse event from {}", queue_name_str);
                            }
                        }

                        let _ = delivery.ack(Default::default()).await;
                    }
                    Err(e) => {
                        let err_msg = format!("Error receiving message from {}: {}", queue_name_str, e);
                        error!("{}", err_msg);
                        let _ = error_tx_clone.send(err_msg);
                        break;
                    }
                }
            }

            let end_msg = format!("Consumer stream ended for {}", queue_name_str);
            warn!("{}", end_msg);
            let _ = error_tx_clone.send(end_msg);
        });

        Ok(())
    }

    /// Handle activity event - IMPLEMENTED
    async fn handle_activity_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        info!("📊 Processing activity event...");

        let payload = event.get("payload").unwrap_or(event);
        
        // Extract fields from event
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        let raw_app_name = payload["app_name"].as_str().unwrap_or("unknown").to_string();
        let raw_window_title = payload["window_title"].as_str().unwrap_or("").to_string();
        let (app_name, window_title) = Self::normalize_activity_names(&raw_app_name, &raw_window_title);
        let duration_seconds = payload["duration_seconds"].as_i64().unwrap_or(0);

        if duration_seconds <= 0 {
            return Ok(());
        }

        // Register device if not exists
        let _ = db.register_device(
            hostname,
            device_id.clone(),
            mac_address,
            None,
        ).await;

        // Insert activity log
        let log = db.insert_activity_log(
            device_id,
            app_name,
            window_title,
            duration_seconds,
        ).await?;

        info!("✅ Activity event stored: {}", log.id);
        Ok(())
    }

    /// Handle inventory event - IMPLEMENTED
    async fn handle_inventory_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        info!("📦 Processing inventory event...");

        let payload = event.get("payload").unwrap_or(event);
        
        // Extract fields
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        
        // Register device if not exists
        let _ = db.register_device(
            hostname,
            device_id.clone(),
            mac_address,
            None,
        ).await;

        // New format: payload.apps = [ { app_name, version, exe_hash, detected_at } ]
        if let Some(apps) = payload["apps"].as_array() {
            for app in apps {
                let app_name = app["app_name"].as_str().unwrap_or("unknown").to_string();
                if Self::should_skip_inventory_app(&app_name) {
                    continue;
                }
                let version = app["version"].as_str().unwrap_or("unknown").to_string();
                let exe_hash = app["exe_hash"].as_str().unwrap_or("unknown").to_string();

                let item = db.insert_inventory(
                    device_id.clone(),
                    app_name,
                    version,
                    exe_hash,
                ).await?;

                info!("✅ Inventory item stored: {}", item.id);
            }
            return Ok(());
        }

        // Backward compatibility format
        if let Some(inventory) = event["inventory"].as_object() {
            for (app_name, details) in inventory {
                if Self::should_skip_inventory_app(app_name) {
                    continue;
                }
                if let Some(obj) = details.as_object() {
                    let version = obj.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                    let exe_hash = obj.get("exe_hash").and_then(|h| h.as_str()).unwrap_or("unknown").to_string();
                    let _ = db.insert_inventory(device_id.clone(), app_name.clone(), version, exe_hash).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_heartbeat_event(
        event: &Value,
        db: &Database,
        config: &RuntimeConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = event.get("payload").unwrap_or(event);
        let event_type = event["event_type"].as_str().unwrap_or("heartbeat");

        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        let status = payload["status"].as_str().unwrap_or("active");

        let _ = db.register_device(hostname, device_id.clone(), mac_address, None).await;

        // Only input_summary events should contribute to persisted time counters.
        // Regular heartbeat events contain cumulative idle_seconds and would inflate totals.
        let event_timestamp = event
            .get("timestamp")
            .and_then(|value| value.as_str())
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);
        let now = Utc::now();
        let max_past = chrono::Duration::seconds(config.max_event_past_skew_seconds.max(1));
        let max_future = chrono::Duration::seconds(config.max_event_future_skew_seconds.max(1));
        if event_timestamp < now - max_past || event_timestamp > now + max_future {
            warn!(
                "Discarding heartbeat/input_summary with out-of-range timestamp. device_id={}, ts={}",
                device_id,
                event_timestamp
            );
            return Ok(());
        }

        let active_seconds = payload["active_seconds"].as_i64().unwrap_or(0);
        let idle_seconds = payload["idle_seconds"].as_i64().unwrap_or(0);
        let keys_count = payload["keys_count"].as_i64().unwrap_or(0);
        let mouse_moves_count = payload["mouse_moves_count"].as_i64().unwrap_or(0);
        let clicks_count = payload["clicks_count"].as_i64().unwrap_or(0);

        if event_type == "input_summary"
            && (active_seconds > 0 || idle_seconds > 0 || keys_count > 0 || mouse_moves_count > 0 || clicks_count > 0)
        {
            let _ = db
                .insert_input_summary(
                    device_id.clone(),
                    event_timestamp,
                    active_seconds,
                    idle_seconds,
                    keys_count,
                    mouse_moves_count,
                    clicks_count,
                    status.to_string(),
                    config.input_bucket_max_seconds,
                )
                .await;
        }

        info!("✅ Heartbeat processed: {} ({})", device_id, status);
        Ok(())
    }

    async fn handle_usb_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let _ = db.register_device(hostname, device_id.clone(), mac_address, None).await;

        let device_name = payload["device_name"].as_str().unwrap_or("USB Device").to_string();
        let action = payload["action"].as_str().unwrap_or("IN").to_string();
        let hardware_id = payload["hardware_id"].as_str().unwrap_or("unknown").to_string();
        let serial_number = payload["serial_number"].as_str().map(str::to_string);
        let volume_label = payload["volume_label"].as_str().map(str::to_string);

        let usb_event = db
            .insert_usb_event(
                device_id.clone(),
                action.clone(),
                hardware_id,
                device_name.clone(),
                serial_number,
                volume_label,
            )
            .await?;

        info!("✅ USB event processed: {} {} {} ({})", device_id, action, device_name, usb_event.id);
        Ok(())
    }

    async fn handle_wifi_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let _ = db.register_device(hostname, device_id.clone(), mac_address, None).await;

        let event_timestamp = event
            .get("timestamp")
            .and_then(|value| value.as_str())
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let interface_name = payload["interface_name"].as_str().unwrap_or("Wi-Fi").to_string();
        let state = payload["state"].as_str().unwrap_or("unknown").to_string();
        let ssid = payload["ssid"].as_str().map(str::to_string);
        let bssid = payload["bssid"].as_str().map(str::to_string);
        let signal_percent = payload["signal_percent"].as_i64().map(|v| v.clamp(0, 100) as i32);

        let wifi_event = db
            .insert_wifi_event(
                device_id.clone(),
                interface_name.clone(),
                state.clone(),
                ssid,
                bssid,
                signal_percent,
                event_timestamp,
            )
            .await?;

        info!("✅ WiFi event processed: {} {} ({})", device_id, interface_name, wifi_event.id);
        Ok(())
    }

    async fn handle_running_apps_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let _ = db.register_device(hostname, device_id.clone(), mac_address, None).await;

        let mut apps = Vec::new();
        if let Some(items) = payload["apps"].as_array() {
            for item in items {
                let app_name = item["app_name"].as_str().unwrap_or("").trim().to_string();
                let primary_title = item["primary_title"].as_str().unwrap_or("").trim().to_string();
                let window_count = item["window_count"].as_i64().unwrap_or(1) as i32;
                let exe_path = item["exe_path"].as_str().map(str::to_string).filter(|value| !value.trim().is_empty());
                let exe_hash = item["exe_hash"].as_str().map(str::to_string).filter(|value| !value.trim().is_empty());

                if Self::should_skip_running_app(&app_name, &primary_title) {
                    continue;
                }

                apps.push((app_name, primary_title, window_count, exe_path, exe_hash));
            }
        }

        db.replace_running_apps_snapshot(device_id.clone(), apps).await?;
        info!("✅ Running apps snapshot processed: {}", device_id);
        Ok(())
    }

    async fn handle_security_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname   = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let _ = db.register_device(hostname, device_id.clone(), mac_address, None).await;

        let query_name = payload["query_name"].as_str().unwrap_or("unknown_query").to_string();
        let query_pack = payload["query_pack"].as_str().map(str::to_string);
        let mitre_technique = payload["mitre_technique"].as_str().map(str::to_string);
        let severity = payload["severity"].as_str().unwrap_or("LOW").to_uppercase();
        let raw_data = payload["raw_data"].clone();
        let event_fingerprint = payload["event_fingerprint"].as_str().map(str::to_string);

        let timestamp = event
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        match db.insert_security_event(
            device_id.clone(),
            query_name.clone(),
            query_pack,
            mitre_technique.clone(),
            severity.clone(),
            raw_data,
            event_fingerprint,
            timestamp,
        ).await {
            Ok(ev) => info!("✅ Security event stored: id={} query={} technique={:?} severity={}",
                ev.id, query_name, mitre_technique, severity),
            Err(e) => warn!("Failed to store security event for {}: {}", device_id, e),
        }

        Ok(())
    }
}
