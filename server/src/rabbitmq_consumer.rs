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
use crate::telemetry_writer::TelemetryWriter;

pub struct RabbitMQConsumer;

pub(crate) static CONSUMER_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

    async fn persist(event: &Value, queue: &str, db: &Database, config: &RuntimeConfig)
        -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        crate::replay_policy::validate(event)?;
        let device = event["device_id"].as_str().ok_or("Missing device_id")?;
        uuid::Uuid::parse_str(device)?;
        let mut writer = TelemetryWriter::begin(&db.pool).await?;
        let fresh = writer.register_processed_event(&Self::event_dedupe_id(event, queue), device,
            event["event_type"].as_str().unwrap_or(queue), event["sequence"].as_i64(),
            event["boot_id"].as_str()).await?;
        if fresh && event.get("source_principal").is_some() {
            writer.insert_session_event(event).await?;
        } else if fresh {
            match queue {
                "activity_queue" => Self::handle_activity_event(event, &mut writer).await?,
                "inventory_queue" => Self::handle_inventory_event(event, &mut writer, config).await?,
                "heartbeat_queue" => Self::handle_heartbeat_event(event, &mut writer, config).await?,
                "usb_queue" => Self::handle_usb_event(event, &mut writer).await?,
                "wifi_queue" => Self::handle_wifi_event(event, &mut writer).await?,
                "running_apps_queue" => Self::handle_running_apps_event(event, &mut writer).await?,
                "security_queue" => Self::handle_security_event(event, &mut writer).await?,
                _ => return Err("Unsupported telemetry queue".into()),
            }
        }
        writer.commit().await?;
        Ok(())
    }

    async fn transfer_failed(channel: &Channel, delivery: &lapin::message::Delivery, queue: &str,
        malformed: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use lapin::types::AMQPValue;
        let mut headers = delivery.properties.headers().clone().unwrap_or_default();
        let attempts = match headers.inner().get("x-monitor-attempts") {
            Some(AMQPValue::LongUInt(value)) => *value,
            _ => 0,
        };
        let destination = if malformed || attempts >= 3 { format!("{queue}.dead") } else { queue.to_string() };
        headers.insert("x-monitor-attempts".into(), AMQPValue::LongUInt(attempts.saturating_add(1)));
        if destination == queue { tokio::time::sleep(std::time::Duration::from_secs(2)).await; }
        let properties = crate::broker_identity::retry_properties(delivery,headers);
        let confirmation = channel.basic_publish("".into(), destination.as_str().into(),
            lapin::options::BasicPublishOptions { mandatory: true, ..Default::default() },
            &delivery.data, properties).await?.await?;
        if !matches!(confirmation, lapin::Confirmation::Ack(None)) {
            return Err("Retry/dead-letter publication was not confirmed or was returned".into());
        }
        if !delivery.ack(Default::default()).await? { return Err("Delivery acknowledgement was not sent".into()); }
        Ok(())
    }

    /// Start RabbitMQ consumer for monitoring events with PostgreSQL database connection
    pub async fn start_consumer(
        rabbitmq_url: &str,
        db: Database,
        config: RuntimeConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("========================================");
        println!("🚀 INICIALIZANDO CONSUMIDOR RABBITMQ");
        println!("========================================");
        info!("Connecting to RabbitMQ");
        
        CONSUMER_READY.store(false, std::sync::atomic::Ordering::Release);
        // Connect to RabbitMQ
        let connection = Connection::connect(
            rabbitmq_url,
            ConnectionProperties::default(),
        )
        .await
        .map_err(|e| {
            error!("❌ Failed to connect to RabbitMQ: {}", e);
            println!("ERROR: No se pudo conectar a RabbitMQ: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        println!("✅ Conectado a RabbitMQ exitosamente");
        info!("✅ Connected to RabbitMQ");

        let channel = connection.create_channel().await
            .map_err(|e| {
                error!("❌ Failed to create channel: {}", e);
                println!("ERROR: No se pudo crear canal RabbitMQ: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        println!("✅ Canal RabbitMQ creado");

        // Declare exchange
        println!("Declarando exchange 'monitoring' (tipo: Topic, durable: true)...");
        info!("📢 Declaring 'monitoring' exchange (Topic, Durable)");
        channel.exchange_declare(
            "monitoring".into(),
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
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
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

        channel.queue_bind("heartbeat_queue".into(), "monitoring".into(), "monitoring.health".into(),
            Default::default(), Default::default()).await?;

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
            })?;

        println!("");
        println!("✅ RabbitMQ Queues initialized");
        println!("========================================");
        info!("✅ RabbitMQ Queues initialized");
        info!("📡 RabbitMQ consumer started, listening to monitoring.* events");

        CONSUMER_READY.store(true, std::sync::atomic::Ordering::Release);
        let stopped = error_rx.recv().await;
        let _ = connection.close(200, "consumer recovery".into()).await;
        CONSUMER_READY.store(false, std::sync::atomic::Ordering::Release);
        if let Some(err_msg) = stopped {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Declare durable queue - with explicit println for debugging
        println!("Intentando declarar cola {}...", queue_name);
        info!("  📋 Creating queue '{}' (Durable: true, Exclusive: false, AutoDelete: false)", queue_name);
        channel.queue_declare(
            queue_name.into(),
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
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        println!("Cola {} declarada con éxito", queue_name);
        if std::env::var("MONITOR_ENV").as_deref()==Ok("production") {
            let scoped=routing_key.replacen("monitoring.","monitoring.*.",1);
            channel.queue_bind(queue_name.into(),"monitoring".into(),scoped.as_str().into(),Default::default(),Default::default()).await?;
            if queue_name=="heartbeat_queue" {channel.queue_bind(queue_name.into(),"monitoring".into(),"monitoring.*.health".into(),Default::default(),Default::default()).await?;}
        }
        info!("    ✅ Queue '{}' created successfully", queue_name);

        // Bind queue to exchange - explicit confirmation
        println!("Vinculando cola {} a exchange 'monitoring' con routing key '{}'", queue_name, routing_key);
        info!("  🔗 Binding '{}' to exchange 'monitoring' with routing key '{}'", queue_name, routing_key);
        channel.queue_bind(
            queue_name.into(),
            "monitoring".into(),
            routing_key.into(),
            Default::default(),
            Default::default(),
        )
        .await
        .map_err(|e| {
            error!("    ❌ Failed to bind queue '{}' to exchange: {}", queue_name, e);
            println!("ERROR al vincular cola {} a exchange: {}", queue_name, e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        println!("Cola {} vinculada con éxito al exchange 'monitoring'", queue_name);
        info!("    ✅ Queue '{}' bound successfully to 'monitoring' with routing key '{}'", queue_name, routing_key);

        channel.basic_qos(16, Default::default()).await?;
        channel.queue_declare(format!("{queue_name}.dead").into(), QueueDeclareOptions { durable: true, ..Default::default() }, Default::default()).await?;
        channel.confirm_select(Default::default()).await?;
        let retry_channel = channel.clone();

        // Start consuming
        let consumer = channel.basic_consume(
            queue_name.into(),
            "".into(),
            BasicConsumeOptions::default(),
            Default::default(),
        )
        .await
        .map_err(|e| {
            error!("    ❌ Failed to start consuming from queue '{}': {}", queue_name, e);
            println!("ERROR al iniciar consumidor de cola {}: {}", queue_name, e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
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
                        if delivery.data.len()>1048576 {let _=delivery.reject(lapin::options::BasicRejectOptions{requeue:false}).await;continue;}
                        let parsed = serde_json::from_slice::<Value>(&delivery.data);
                        if let Ok(event)=&parsed {
                            match crate::broker_identity::validate_delivery(&db_clone,&delivery,event).await {
                                Ok(true)=>{},
                                Ok(false)=>{warn!("Quarantined telemetry with invalid/revoked device identity");if Self::transfer_failed(&retry_channel,&delivery,&queue_name_str,true).await.is_err(){let _=error_tx_clone.send("Identity quarantine unavailable".into());break;}continue;},
                                Err(_)=>{let _=error_tx_clone.send("Identity database unavailable".into());break;}
                            }
                        }
                        let malformed = parsed.as_ref().map_or(true, |event| {
                            event["device_id"].as_str().and_then(|id| uuid::Uuid::parse_str(id).ok()).is_none()
                                || crate::replay_policy::validate(event).is_err()
                        });
                        let result = match parsed {
                            Ok(event) if !malformed => Self::persist(&event, &queue_name_str, &db_clone, &config_clone).await,
                            _ => Err("Malformed telemetry".into()),
                        };
                        let settled = match result {
                            Ok(()) => delivery.ack(Default::default()).await.map_err(|e| e.to_string()).and_then(|sent| if sent { Ok(()) } else { Err("Delivery acknowledgement was not sent".into()) }),
                            Err(error) => {
                                warn!(queue = %queue_name_str, %error, "Telemetry transaction failed; retaining message in retry/dead queue");
                                Self::transfer_failed(&retry_channel, &delivery, &queue_name_str, malformed).await.map_err(|e| e.to_string())
                            }
                        };
                        if let Err(error) = settled {
                            // Closing the connection requeues the unacknowledged original.
                            let _ = error_tx_clone.send(error);
                            break;
                        }

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
    async fn handle_activity_event(event: &Value, db: &mut TelemetryWriter) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(
            hostname,
            device_id.clone(),
            mac_address,
            None,
            version,
        ).await?;

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
    async fn handle_inventory_event(event: &Value, db: &mut TelemetryWriter, config: &RuntimeConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("📦 Processing inventory event...");

        let payload = event.get("payload").unwrap_or(event);
        
        // Extract fields
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        
        // Register device if not exists
        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(
            hostname,
            device_id.clone(),
            mac_address,
            None,
            version,
        ).await?;

        if event["event_type"]=="inventory_snapshot" {
            if event["schema_version"].as_u64()!=Some(2) || event["source_role"]=="session" {return Err("Invalid inventory snapshot role/version".into());}
            let observed=DateTime::parse_from_rfc3339(event["timestamp"].as_str().ok_or("Missing inventory timestamp")?)?.with_timezone(&Utc);
            if observed>Utc::now()+chrono::Duration::seconds(config.max_event_future_skew_seconds.max(1)) {return Err("Future inventory snapshot".into());}
            return db.reconcile_inventory(&device_id,observed,payload).await;
        }

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
        db: &mut TelemetryWriter,
        config: &RuntimeConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = event.get("payload").unwrap_or(event);
        let event_type = event["event_type"].as_str().unwrap_or("heartbeat");

        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        let status = payload["status"].as_str().unwrap_or("active");

        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(hostname, device_id.clone(), mac_address, None, version).await?;

        // Only input_summary events should contribute to persisted time counters.
        // Regular heartbeat events contain cumulative idle_seconds and would inflate totals.
        let parsed_timestamp = event
            .get("timestamp")
            .and_then(|value| value.as_str())
            .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
            .map(|dt| dt.with_timezone(&Utc));
        if (event_type == "agent_health" || payload.get("system_metrics").is_some()) && parsed_timestamp.is_none() {
            return Err("Current telemetry requires a valid observed timestamp".into());
        }
        let event_timestamp = parsed_timestamp.unwrap_or_else(Utc::now);
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

        if event_type == "agent_health" {
            crate::telemetry_contract::health(payload)?;
            let health = serde_json::json!({"status":payload["status"], "tasks":payload["tasks"], "collectors":payload["collectors"],
                "capabilities":payload["capabilities"], "monitoring_paused":payload["monitoring_paused"],
                "enabled_monitors":payload["enabled_monitors"], "disabled_monitors":payload["disabled_monitors"],
                "agent_uptime_seconds":payload["agent_uptime_seconds"], "cache_unsynced_events":payload["cache_unsynced_events"],
                "sessions":payload["sessions"], "policy_fingerprint":payload["policy_fingerprint"], "config_version":payload["config_version"], "version":payload["version"]});
            db.update_telemetry_state(&device_id, "health", event_timestamp, health).await?;
            return Ok(());
        }
        if event_type == "input_summary" && event["source_role"] != "session" {
            if let Some(metrics) = payload.get("system_metrics") {
                crate::telemetry_contract::resources(metrics)?;
                db.update_telemetry_state(&device_id, "resources", event_timestamp, metrics.clone()).await?;
            }
        }

        let active_seconds = payload["active_seconds"].as_i64().unwrap_or(0);
        let idle_seconds = payload["idle_seconds"].as_i64().unwrap_or(0);
        let keys_count = payload["keys_count"].as_i64().unwrap_or(0);
        let mouse_moves_count = payload["mouse_moves_count"].as_i64().unwrap_or(0);
        let clicks_count = payload["clicks_count"].as_i64().unwrap_or(0);
        let cpu_percent = payload["cpu_percent"].as_f64();
        let memory_used_mb = payload["memory_used_mb"].as_f64();
        let memory_percent = payload["memory_percent"].as_f64();
        let top_process_name = payload["top_process_name"].as_str().map(|value| value.to_string());
        let top_process_cpu_percent = payload["top_process_cpu_percent"].as_f64();
        let top_process_memory_mb = payload["top_process_memory_mb"].as_f64();

        if event_type == "input_summary"
            && (active_seconds > 0 || idle_seconds > 0 || keys_count > 0 || mouse_moves_count > 0 || clicks_count > 0 || (payload["is_service"] != true && payload["input_available"].is_boolean()))
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
                    payload["input_available"].as_bool(),
                )
                .await?;
        }

        if event_type == "input_summary" && cpu_percent.is_some() && memory_used_mb.is_some() && memory_percent.is_some() {
            let _ = db
                .insert_node_resource_metric(
                    device_id.clone(),
                    event_timestamp,
                    cpu_percent.unwrap_or(0.0),
                    memory_used_mb.unwrap_or(0.0),
                    memory_percent.unwrap_or(0.0),
                    top_process_name,
                    top_process_cpu_percent,
                    top_process_memory_mb,
                )
                .await?;
        }

        info!("✅ Heartbeat processed: {} ({})", device_id, status);
        Ok(())
    }

    async fn handle_usb_event(event: &Value, db: &mut TelemetryWriter) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(hostname, device_id.clone(), mac_address, None, version).await?;

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
                event["timestamp"].as_str().and_then(|v|DateTime::parse_from_rfc3339(v).ok()).map(|v|v.with_timezone(&Utc)).unwrap_or_else(Utc::now),
            )
            .await?;

        info!("✅ USB event processed: {} {} {} ({})", device_id, action, device_name, usb_event.id);
        Ok(())
    }

    async fn handle_wifi_event(event: &Value, db: &mut TelemetryWriter) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(hostname, device_id.clone(), mac_address, None, version).await?;

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

    async fn handle_running_apps_event(event: &Value, db: &mut TelemetryWriter) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);

        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(hostname, device_id.clone(), mac_address, None, version).await?;

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

    async fn handle_security_event(event: &Value, db: &mut TelemetryWriter) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = event.get("payload").unwrap_or(event);
        let device_id = event["device_id"].as_str().unwrap_or("unknown").to_string();
        let hostname   = event["hostname"].as_str().unwrap_or(&device_id).to_string();
        let mac_address = event["mac_address"].as_str().map(str::to_string);
        let timestamp = event
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let version = event.get("version").and_then(|v| v.as_str()).map(str::to_string);
        let _ = db.register_device(hostname, device_id.clone(), mac_address, None, version).await?;

        let query_name = payload["query_name"].as_str();
        let inferred_alert_type = match query_name {
            Some("usb_file_copy_detected") => Some("USB_FILE_COPY_DETECTED"),
            _ => None,
        };

        if let Some(alert_type) = payload["alert_type"].as_str().or(inferred_alert_type) {
            let severity = payload["severity"].as_str().unwrap_or("CRITICAL").to_uppercase();
            let usb_raw = payload.get("raw_data").unwrap_or(payload);
            let usb_file = usb_raw["file_name"].as_str().unwrap_or("archivo");
            let usb_drive = usb_raw["drive_letter"].as_str().unwrap_or("USB");
            let usb_size = usb_raw["size_bytes"].as_i64().unwrap_or(0);
            let default_usb_description = format!(
                "Copia a USB detectada: {} en {} ({} bytes)",
                usb_file,
                usb_drive,
                usb_size
            );
            let description = payload["description"]
                .as_str()
                .or_else(|| payload["message"].as_str())
                .unwrap_or_else(|| {
                    if alert_type == "USB_FILE_COPY_DETECTED" {
                        default_usb_description.as_str()
                    } else {
                        "Security alert received from agent"
                    }
                })
                .to_string();

            if let Ok(device_uuid) = uuid::Uuid::parse_str(&device_id) {
                db
                    .insert_security_alert(
                        device_uuid,
                        alert_type,
                        payload["app_name"].as_str().or_else(|| usb_raw["source"].as_str()),
                        payload["exe_hash"].as_str(),
                        description.as_str(),
                        severity.as_str(),
                    )
                    .await?;

                if alert_type == "PROCESS_TERMINATION_ATTEMPTED" {
                    let blocked = payload["blocked"].as_bool().unwrap_or(true);
                    let auto_restarted = payload["auto_restarted"].as_bool().unwrap_or(true);
                    let _ = db
                        .insert_process_termination_attempt(
                            device_uuid,
                            payload["method"].as_str().unwrap_or("unknown"),
                            payload["attempted_by"].as_str(),
                            blocked,
                            auto_restarted,
                            Some(timestamp),
                        )
                        .await?;
                }
            } else {
                warn!(
                    "Skipping security alert persistence due to invalid device_id: {}",
                    device_id
                );
            }

            if query_name.is_none() {
                return Ok(());
            }
        }

        let query_name = query_name.unwrap_or("unknown_query").to_string();
        let query_pack = payload["query_pack"].as_str().map(str::to_string);
        let mitre_technique = payload["mitre_technique"].as_str().map(str::to_string);
        let severity = payload["severity"].as_str().unwrap_or("LOW").to_uppercase();
        let raw_data = payload["raw_data"].clone();
        let event_fingerprint = payload["event_fingerprint"].as_str().map(str::to_string);

        db.insert_security_event(
            device_id.clone(),
            query_name.clone(),
            query_pack,
            mitre_technique.clone(),
            severity.clone(),
            raw_data,
            event_fingerprint,
            timestamp,
        ).await?;

        Ok(())
    }
}

#[cfg(test)]
mod transaction_tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires disposable TEST_DATABASE_URL"]
    async fn rollback_and_concurrent_replay() {
        let url = std::env::var("TEST_DATABASE_URL").unwrap();
        let admin = sqlx::PgPool::connect(&url).await.unwrap();
        let name = format!("telemetry_test_{}", uuid::Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE DATABASE {name}")).execute(&admin).await.unwrap();
        let test_url = format!("{}/{}", url.rsplit_once('/').unwrap().0, name);
        let db = Database::connect(&test_url).await.unwrap();
        let config = RuntimeConfig::from_env();
        let device = uuid::Uuid::new_v4().to_string();
        let event_id = uuid::Uuid::new_v4().to_string();
        let mut event = serde_json::json!({"event_id":event_id,"device_id":device,"hostname":"transaction-test",
            "event_type":"input_summary", "timestamp":Utc::now(),
            "payload":{"active_seconds":30,"cpu_percent":42.5,"memory_used_mb":1024,"memory_percent":25}});
        sqlx::query("ALTER TABLE node_resource_metrics ADD CONSTRAINT injected_failure CHECK (cpu_percent < 0)")
            .execute(&db.pool).await.unwrap();
        assert!(RabbitMQConsumer::persist(&event, "heartbeat_queue", &db, &config).await.is_err());
        for table in ["devices", "processed_events", "input_activity_metrics", "node_resource_metrics"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}")).fetch_one(&db.pool).await.unwrap();
            assert_eq!(count, 0, "Rollback must include {table}");
        }
        sqlx::query("ALTER TABLE node_resource_metrics DROP CONSTRAINT injected_failure").execute(&db.pool).await.unwrap();
        let results = futures::future::join_all((0..16).map(|_| RabbitMQConsumer::persist(&event, "heartbeat_queue", &db, &config))).await;
        assert!(results.into_iter().all(|r| r.is_ok()));
        for table in ["devices", "processed_events", "input_activity_metrics", "node_resource_metrics"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}")).fetch_one(&db.pool).await.unwrap();
            assert_eq!(count, 1, "Concurrent replay duplicated {table}");
        }
        event["event_id"] = uuid::Uuid::new_v4().to_string().into();
        RabbitMQConsumer::persist(&event, "heartbeat_queue", &db, &config).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM node_resource_metrics").fetch_one(&db.pool).await.unwrap();
        assert_eq!(count, 2);
        db.pool.close().await;
        sqlx::query(&format!("DROP DATABASE {name}")).execute(&admin).await.unwrap();
        admin.close().await;
    }
}

#[cfg(test)]
mod broker_tests {
    use super::*;
    use lapin::{options::*, BasicProperties};
    #[tokio::test]
    #[ignore = "requires disposable TEST_DATABASE_URL and local test RabbitMQ management"]
    async fn failed_write_retries_to_dead_queue_then_replays_after_repair() {
        let url = std::env::var("TEST_DATABASE_URL").unwrap();
        let management = std::env::var("TEST_RABBITMQ_MANAGEMENT_URL").unwrap();
        let broker = std::env::var("TEST_RABBITMQ_BASE_URL").unwrap();
        let name = format!("delivery_test_{}", uuid::Uuid::new_v4().simple());
        let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().unwrap();
        let vhost_url = format!("{management}/api/vhosts/{name}");
        http.put(&vhost_url).basic_auth("guest", Some("guest")).send().await.unwrap().error_for_status().unwrap();
        let admin = sqlx::PgPool::connect(&url).await.unwrap();
        sqlx::query(&format!("CREATE DATABASE {name}")).execute(&admin).await.unwrap();
        let db_url = format!("{}/{}",url.rsplit_once('/').unwrap().0,name);
        let db = Database::connect(&db_url).await.unwrap();
        let amqp_url = format!("{broker}/{name}");
        let consumer_db = db.clone(); let consumer_url = amqp_url.clone();
        let worker = tokio::spawn(async move { RabbitMQConsumer::start_consumer(&consumer_url, consumer_db, RuntimeConfig::from_env()).await });
        let conn = Connection::connect(&amqp_url, ConnectionProperties::default()).await.unwrap();
        let channel = conn.create_channel().await.unwrap();
        channel.confirm_select(Default::default()).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while !CONSUMER_READY.load(std::sync::atomic::Ordering::Acquire) { tokio::time::sleep(std::time::Duration::from_millis(50)).await; }
        }).await.unwrap();
        sqlx::query("ALTER TABLE node_resource_metrics ADD CONSTRAINT injected_failure CHECK(cpu_percent < 0)").execute(&db.pool).await.unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let event = serde_json::json!({"event_id":id,"device_id":uuid::Uuid::new_v4(),"hostname":"retry-test",
            "event_type":"input_summary","timestamp":Utc::now(),"payload":{"active_seconds":30,"cpu_percent":42.5,"memory_used_mb":1024,"memory_percent":25}});
        let body = serde_json::to_vec(&event).unwrap();
        let confirmed = channel.basic_publish("monitoring".into(),"monitoring.heartbeat".into(), BasicPublishOptions { mandatory:true,..Default::default() },
            &body,BasicProperties::default().with_delivery_mode(2)).await.unwrap().await.unwrap();
        assert!(matches!(confirmed,lapin::Confirmation::Ack(None)));
        let dead = tokio::time::timeout(std::time::Duration::from_secs(40), async {
            loop {
                if let Some(message) = channel.basic_get("heartbeat_queue.dead".into(),BasicGetOptions::default()).await.unwrap() { break message; }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }).await.unwrap();
        assert_eq!(dead.delivery.data,body);
        assert_eq!(dead.delivery.properties.headers().as_ref().unwrap().inner().get("x-monitor-attempts"), Some(&lapin::types::AMQPValue::LongUInt(4)));
        for table in ["processed_events","devices","input_activity_metrics","node_resource_metrics"] {
            let count:i64=sqlx::query_scalar(&format!("SELECT count(*) FROM {table}")).fetch_one(&db.pool).await.unwrap(); assert_eq!(count,0);
        }
        sqlx::query("ALTER TABLE node_resource_metrics DROP CONSTRAINT injected_failure").execute(&db.pool).await.unwrap();
        for _ in 0..8 {
            channel.basic_publish("monitoring".into(),"monitoring.heartbeat".into(),Default::default(), &body,BasicProperties::default().with_delivery_mode(2)).await.unwrap().await.unwrap();
        }
        dead.delivery.ack(Default::default()).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM node_resource_metrics").fetch_one(&db.pool).await.unwrap();
                if count==1 { break; }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }).await.unwrap();
        // Malformed input is retained in dead-letter storage, not silently acknowledged.
        channel.basic_publish("monitoring".into(),"monitoring.heartbeat".into(),Default::default(), b"not-json",BasicProperties::default().with_delivery_mode(2)).await.unwrap().await.unwrap();
        let malformed = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                if let Some(message)=channel.basic_get("heartbeat_queue.dead".into(),Default::default()).await.unwrap() { break message; }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }).await.unwrap();
        assert_eq!(malformed.delivery.data,b"not-json"); malformed.delivery.ack(Default::default()).await.unwrap();
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM node_resource_metrics").fetch_one(&db.pool).await.unwrap(); assert_eq!(count,1);
        conn.close(200,"test complete".into()).await.unwrap();
        worker.abort(); let _ = worker.await;
        db.pool.close().await;
        http.delete(&vhost_url).basic_auth("guest",Some("guest")).send().await.unwrap().error_for_status().unwrap();
        sqlx::query(&format!("DROP DATABASE {name} WITH (FORCE)")).execute(&admin).await.unwrap();
        admin.close().await;
    }
}
