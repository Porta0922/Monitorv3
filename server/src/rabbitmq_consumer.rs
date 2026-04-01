// RabbitMQ consumer for agent telemetry
use lapin::{Connection, ConnectionProperties, Channel};
use lapin::options::QueueDeclareOptions;
use lapin::options::BasicConsumeOptions;
use serde_json::Value;
use tracing::{info, error, warn};

pub struct RabbitMQConsumer;

impl RabbitMQConsumer {
    /// Start RabbitMQ consumer for monitoring events
    pub async fn start_consumer(rabbitmq_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔌 Connecting to RabbitMQ at: {}", rabbitmq_url);
        
        // Connect to RabbitMQ
        let connection = Connection::connect(
            rabbitmq_url,
            ConnectionProperties::default(),
        )
        .await
        .map_err(|e| {
            error!("❌ Failed to connect to RabbitMQ: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        info!("✅ Connected to RabbitMQ");

        let channel = connection.create_channel().await
            .map_err(|e| {
                error!("❌ Failed to create channel: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;

        // Declare exchange
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
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        info!("✅ Exchange 'monitoring' declared successfully");

        // Create queues and bind them
        info!("🏗️  Creating queues...");
        Self::setup_queue(&channel, "activity_logs", "monitoring.activity").await
            .map_err(|e| {
                error!("❌ Failed to setup activity_logs queue: {}", e);
                e
            })?;
        
        Self::setup_queue(&channel, "inventory_logs", "monitoring.inventory").await
            .map_err(|e| {
                error!("❌ Failed to setup inventory_logs queue: {}", e);
                e
            })?;
        
        Self::setup_queue(&channel, "security_alerts", "monitoring.security").await
            .map_err(|e| {
                error!("❌ Failed to setup security_alerts queue: {}", e);
                e
            })?;

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
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Declare durable queue
        info!("  📋 Creating queue '{}' (Durable: true)", queue_name);
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
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        info!("    ✅ Queue '{}' created", queue_name);

        // Bind queue to exchange
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
            error!("    ❌ Failed to bind queue '{}': {}", queue_name, e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        info!("    ✅ Queue '{}' bound successfully", queue_name);

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
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        info!("    🎧 Consumer started for queue '{}'", queue_name);

        // Spawn consumer task
        let channel_clone = channel.clone();
        let queue_name = queue_name.to_string();
        
        tokio::spawn(async move {
            use futures::stream::StreamExt;
            
            let mut consumer = consumer;
            while let Some(delivery) = consumer.next().await {
                if let Ok(delivery) = delivery {
                    if let Ok(payload) = std::str::from_utf8(&delivery.data) {
                        if let Ok(event) = serde_json::from_str::<Value>(payload) {
                            match queue_name.as_str() {
                                "activity_logs" => {
                                    Self::handle_activity_event(&event).await;
                                }
                                "inventory_logs" => {
                                    Self::handle_inventory_event(&event).await;
                                }
                                "security_alerts" => {
                                    Self::handle_security_event(&event).await;
                                }
                                _ => {}
                            }
                        } else {
                            warn!("Failed to parse event from {}", queue_name);
                        }
                    }
                    
                    // Acknowledge message
                    let _ = delivery.ack(Default::default()).await;
                } else {
                    error!("Error receiving message from {}", queue_name);
                }
            }
        });

        Ok(())
    }

    /// Handle activity event
    async fn handle_activity_event(event: &Value) {
        info!("Activity event received: {:?}", event);
        // TODO: Parse event and insert into activity_logs table
        // TODO: Validate device_id exists
        // TODO: Extract app_name, window_title, duration_seconds
        // TODO: INSERT into activity_logs hypertable
    }

    /// Handle inventory event
    async fn handle_inventory_event(event: &Value) {
        info!("Inventory event received: {:?}", event);
        // TODO: Parse event and insert into app_inventory table
        // TODO: Validate executable hash
        // TODO: Check against whitelist
        // TODO: Flag hash mismatches as security alerts
    }

    /// Handle security event
    async fn handle_security_event(event: &Value) {
        warn!("Security event received: {:?}", event);
        // TODO: Parse event and insert into security_alerts table
        // TODO: Set severity level
        // TODO: Potentially trigger notifications
    }
}
