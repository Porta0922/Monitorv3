// RabbitMQ publisher for agent events
use lapin::{Connection, ConnectionProperties, Channel};
use lapin::options::BasicPublishOptions;
use lapin::BasicProperties;
use uuid::Uuid;
use serde_json::json;

use crate::usb_detection::UsbEvent;

pub struct RabbitMQPublisher {
    channel: Channel,
}

impl RabbitMQPublisher {
    /// Connect to RabbitMQ and initialize exchanges
    pub async fn connect(rabbitmq_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Connect to RabbitMQ
        let connection = Connection::connect(
            rabbitmq_url,
            ConnectionProperties::default(),
        )
        .await?;

        let channel = connection.create_channel().await?;

        // Declare topic exchange for monitoring events
        channel.exchange_declare(
            "monitoring",
            lapin::ExchangeKind::Topic,
            lapin::options::ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            Default::default(),
        )
        .await?;

        Ok(Self { channel })
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
    async fn publish_event(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let routing_key = format!("monitoring.{}", event_type);
        let body = serde_json::to_vec(&payload)?;

        self.channel.basic_publish(
            "monitoring",
            &routing_key,
            BasicPublishOptions::default(),
            &body,
            BasicProperties::default()
                .with_content_type("application/json".into())
                .with_delivery_mode(lapin::types::AMQPValue::ShortUInt(2)), // Persistent
        )
        .await?
        .await?;

        tracing::debug!("Published event: {} ({} bytes)", routing_key, body.len());
        Ok(())
    }

    /// Health check connection
    pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Try to declare exchange (lightweight operation)
        self.channel.exchange_declare(
            "monitoring",
            lapin::ExchangeKind::Topic,
            lapin::options::ExchangeDeclareOptions {
                durable: true,
                passive: true,  // Don't create, just check
                ..Default::default()
            },
            Default::default(),
        )
        .await
        .map(|_| true)
        .map_err(|e| e.into())
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
