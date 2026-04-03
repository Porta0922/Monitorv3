// RabbitMQ publisher for agent events
use lapin::{Connection, ConnectionProperties, Channel};
use lapin::options::BasicPublishOptions;
use lapin::BasicProperties;
use std::io;
use tokio::sync::Mutex;
use uuid::Uuid;
use serde_json::json;

use crate::usb_detection::UsbEvent;

pub struct RabbitMQPublisher {
    rabbitmq_url: String,
    state: Mutex<PublisherState>,
}

struct PublisherState {
    connection: Option<Connection>,
    channel: Option<Channel>,
}

impl RabbitMQPublisher {
    /// Connect to RabbitMQ and initialize exchanges
    pub async fn connect(rabbitmq_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let publisher = Self {
            rabbitmq_url: rabbitmq_url.to_string(),
            state: Mutex::new(PublisherState {
                connection: None,
                channel: None,
            }),
        };

        {
            let mut state = publisher.state.lock().await;
            Self::connect_locked(&publisher.rabbitmq_url, &mut state).await?;
        }

        Ok(publisher)
    }

    async fn connect_locked(
        rabbitmq_url: &str,
        state: &mut PublisherState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("🔌 Agent connecting to RabbitMQ at: {}", rabbitmq_url);

        let connection = Connection::connect(rabbitmq_url, ConnectionProperties::default())
            .await
            .map_err(|e| {
                tracing::error!("❌ Agent failed to connect to RabbitMQ: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;

        let channel = connection.create_channel().await.map_err(|e| {
            tracing::error!("❌ Agent failed to create channel: {}", e);
            Box::new(e) as Box<dyn std::error::Error>
        })?;

        tracing::info!("📢 Agent declaring 'monitoring' exchange (Topic, Durable)");
        channel
            .exchange_declare(
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
                tracing::error!("❌ Agent failed to declare exchange: {}", e);
                Box::new(e) as Box<dyn std::error::Error>
            })?;

        state.connection = Some(connection);
        state.channel = Some(channel);

        tracing::info!("✅ Agent RabbitMQ connection/channel ready");
        Ok(())
    }

    fn boxed_error(message: String) -> Box<dyn std::error::Error> {
        Box::new(io::Error::new(io::ErrorKind::Other, message))
    }

    /// Publish activity event
    pub async fn publish_activity_event(
        &self,
        device_id: Uuid,
        app_name: &str,
        window_title: &str,
        duration_seconds: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "device_id": device_id.to_string(),
            "app_name": app_name,
            "window_title": window_title,
            "duration_seconds": duration_seconds,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish_event("activity", payload).await
    }

    /// Publish inventory event
    pub async fn publish_inventory_event(
        &self,
        device_id: Uuid,
        inventory_data: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "device_id": device_id.to_string(),
            "inventory": inventory_data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish_event("inventory", payload).await
    }

    /// Publish USB event
    pub async fn publish_usb_event(&self, event: &UsbEvent) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "device_id": event.device_id.to_string(),
            "usb_device": {
                "device_id": event.usb_device.device_id,
                "vendor_id": event.usb_device.vendor_id,
                "product_id": event.usb_device.product_id,
                "serial_number": event.usb_device.serial_number,
                "device_name": event.usb_device.device_name,
                "volume_label": event.usb_device.volume_label,
                "capacity_bytes": event.usb_device.capacity_bytes,
            },
            "action": match event.action {
                crate::usb_detection::UsbAction::Connected => "IN",
                crate::usb_detection::UsbAction::Disconnected => "OUT",
            },
            "timestamp": event.timestamp.to_rfc3339(),
        });

        self.publish_event("usb", payload).await
    }

    /// Publish security event
    pub async fn publish_security_event(
        &self,
        device_id: Uuid,
        alert_type: &str,
        app_name: &str,
        exe_hash: &str,
        description: &str,
        severity: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "device_id": device_id.to_string(),
            "alert_type": alert_type,
            "app_name": app_name,
            "exe_hash": exe_hash,
            "description": description,
            "severity": severity,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish_event("security", payload).await
    }

    /// Generic event publisher
    pub async fn publish_event(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let routing_key = format!("monitoring.{}", event_type);
        let body = serde_json::to_vec(&payload)?;

        tracing::info!("📤 Publishing event: {} (routing_key: {})", event_type, routing_key);

        let mut last_error: Option<String> = None;

        for attempt in 1..=2 {
            {
                let mut state = self.state.lock().await;
                if state.channel.is_none() {
                    Self::connect_locked(&self.rabbitmq_url, &mut state).await?;
                }
            }

            let channel = {
                let state = self.state.lock().await;
                state.channel.clone().ok_or_else(|| {
                    Self::boxed_error("RabbitMQ channel unavailable after reconnect".to_string())
                })?
            };

            let publish_result = async {
                channel
                    .basic_publish(
                        "monitoring",
                        &routing_key,
                        BasicPublishOptions::default(),
                        &body,
                        BasicProperties::default()
                            .with_content_type("application/json".into())
                            .with_delivery_mode(2u8),
                    )
                    .await?
                    .await?;
                Ok::<(), lapin::Error>(())
            }
            .await;

            match publish_result {
                Ok(_) => {
                    tracing::info!("✅ Event published successfully: {} ({} bytes)", routing_key, body.len());
                    return Ok(());
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    tracing::warn!(
                        "Publish attempt {} failed for {}: {}",
                        attempt,
                        routing_key,
                        err_msg
                    );
                    last_error = Some(err_msg);

                    let mut state = self.state.lock().await;
                    state.channel = None;
                    state.connection = None;
                }
            }
        }

        Err(Self::boxed_error(format!(
            "Publish failed after reconnect retry for {}: {}",
            routing_key,
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )))
    }

    /// Health check connection
    pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut state = self.state.lock().await;
        if state.channel.is_none() {
            if Self::connect_locked(&self.rabbitmq_url, &mut state).await.is_err() {
                return Ok(false);
            }
        }

        let Some(channel) = state.channel.clone() else {
            return Ok(false);
        };

        match channel
            .exchange_declare(
                "monitoring",
                lapin::ExchangeKind::Topic,
                lapin::options::ExchangeDeclareOptions {
                    durable: true,
                    passive: true,
                    ..Default::default()
                },
                Default::default(),
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => {
                state.channel = None;
                state.connection = None;
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]  // Requires running RabbitMQ
    async fn test_rabbitmq_connection() {
        let publisher = RabbitMQPublisher::connect("amqp://guest:guest@localhost:5672")
            .await;
        assert!(publisher.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_publish_activity_event() {
        let publisher = RabbitMQPublisher::connect("amqp://guest:guest@localhost:5672")
            .await
            .expect("Failed to connect");

        let device_id = Uuid::new_v4();
        let result = publisher.publish_activity_event(
            device_id,
            "notepad.exe",
            "Untitled - Notepad",
            60,
        )
        .await;

        assert!(result.is_ok());
    }
}
