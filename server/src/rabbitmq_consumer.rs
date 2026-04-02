// RabbitMQ consumer for agent telemetry
use lapin::{Connection, ConnectionProperties, Channel};
use lapin::options::QueueDeclareOptions;
use lapin::options::BasicConsumeOptions;
use serde_json::Value;
use tracing::{info, error, warn};

use crate::postgres_db::Database;

pub struct RabbitMQConsumer;

impl RabbitMQConsumer {
    /// Start RabbitMQ consumer for monitoring events with PostgreSQL database connection
    pub async fn start_consumer(rabbitmq_url: &str, db: Database) -> Result<(), Box<dyn std::error::Error>> {
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

        // Create queues and bind them (ignoring any .env queue config, always create standard queues)
        println!("");
        println!("CREANDO Y VINCULANDO COLAS ESTÁNDAR...");
        info!("🏗️  Creating standard queues (ignoring .env queue configuration)...");
        Self::setup_queue(&channel, "inventory_queue", "monitoring.inventory", db.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup inventory_queue: {}", e);
                println!("ERROR CRÍTICO: inventory_queue falló");
                e
            })?;
        
        Self::setup_queue(&channel, "activity_queue", "monitoring.activity", db.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup activity_queue: {}", e);
                println!("ERROR CRÍTICO: activity_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "heartbeat_queue", "monitoring.heartbeat", db.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup heartbeat_queue: {}", e);
                println!("ERROR CRÍTICO: heartbeat_queue falló");
                e
            })?;

        Self::setup_queue(&channel, "usb_queue", "monitoring.usb", db.clone()).await
            .map_err(|e| {
                error!("❌ Failed to setup usb_queue: {}", e);
                println!("ERROR CRÍTICO: usb_queue falló");
                e
            })?;

        println!("");
        println!("✅ RabbitMQ Queues initialized");
        println!("========================================");
        info!("✅ RabbitMQ Queues initialized");
        info!("📡 RabbitMQ consumer started, listening to monitoring.* events");

        // Keep connection alive
        tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;

        Ok(())
    }

    /// Setup queue and bind to exchange
    async fn setup_queue(
        channel: &Channel,
        queue_name: &str,
        routing_key: &str,
        db: Database,
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
        
        tokio::spawn(async move {
            use futures::stream::StreamExt;
            
            let mut consumer = consumer;
            while let Some(delivery) = consumer.next().await {
                if let Ok(delivery) = delivery {
                    if let Ok(payload) = std::str::from_utf8(&delivery.data) {
                        if let Ok(event) = serde_json::from_str::<Value>(payload) {
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
                                    if let Err(e) = Self::handle_heartbeat_event(&event, &db_clone).await {
                                        warn!("Failed to process heartbeat event: {}", e);
                                    }
                                }
                                "usb_queue" => {
                                    if let Err(e) = Self::handle_usb_event(&event, &db_clone).await {
                                        warn!("Failed to process usb event: {}", e);
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            warn!("Failed to parse event from {}", queue_name_str);
                        }
                    }
                    
                    // Acknowledge message
                    let _ = delivery.ack(Default::default()).await;
                } else {
                    error!("Error receiving message from {}", queue_name_str);
                }
            }
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
        let app_name = payload["app_name"].as_str().unwrap_or("unknown").to_string();
        let window_title = payload["window_title"].as_str().unwrap_or("").to_string();
        let duration_seconds = payload["duration_seconds"].as_i64().unwrap_or(0);

        if app_name == "unknown" || duration_seconds <= 0 {
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
                if let Some(obj) = details.as_object() {
                    let version = obj.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                    let exe_hash = obj.get("exe_hash").and_then(|h| h.as_str()).unwrap_or("unknown").to_string();
                    let _ = db.insert_inventory(device_id.clone(), app_name.clone(), version, exe_hash).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_heartbeat_event(event: &Value, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let payload = event.get("payload").unwrap_or(event);

        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        let status = payload["status"].as_str().unwrap_or("active");

        let _ = db.register_device(hostname, device_id.clone(), mac_address, None).await;

        // Input summary events carry active/idle minute counters.
        let active_seconds = payload["active_seconds"].as_i64().unwrap_or(0);
        let idle_seconds = payload["idle_seconds"].as_i64().unwrap_or(0);
        if active_seconds > 0 || idle_seconds > 0 {
            let _ = db
                .insert_input_summary(
                    device_id.clone(),
                    active_seconds,
                    idle_seconds,
                    status.to_string(),
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
}
