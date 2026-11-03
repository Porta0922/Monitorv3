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
    pub async fn connect(rabbitmq_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if crate::network_identity::production() && reqwest::Url::parse(rabbitmq_url).map(|u|u.scheme()!="amqps").unwrap_or(true){return Err("Production requires AMQPS".into());}
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Agent connecting to RabbitMQ");

        let connection_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            Connection::connect(rabbitmq_url, ConnectionProperties::default())
        ).await;

        let connection = match connection_result {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                tracing::error!("❌ Agent failed to connect to RabbitMQ: {}", e);
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            }
            Err(_) => {
                let err_msg = "RabbitMQ connection timed out after 10s";
                tracing::error!("❌ {}", err_msg);
                return Err(Self::boxed_error(err_msg.to_string()));
            }
        };

        let channel = connection.create_channel().await.map_err(|e| {
            tracing::error!("❌ Agent failed to create channel: {}", e);
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        if !crate::network_identity::production() {
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
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        }
        channel.confirm_select(lapin::options::ConfirmSelectOptions::default()).await?;

        state.connection = Some(connection);
        state.channel = Some(channel);

        tracing::info!("✅ Agent RabbitMQ connection/channel ready");
        Ok(())
    }

    fn boxed_error(message: String) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(io::Error::new(io::ErrorKind::Other, message))
    }

    /// Publish activity event
    pub async fn publish_activity_event(
        &self,
        device_id: Uuid,
        app_name: &str,
        window_title: &str,
        duration_seconds: i64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({
            "device_id": device_id.to_string(),
            "inventory": inventory_data,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.publish_event("inventory", payload).await
    }

    /// Publish USB event
    pub async fn publish_usb_event(&self, event: &UsbEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut payload=payload;
        if let Some(object)=payload.as_object_mut(){object.remove("auth_token");}
        let routing_key=if crate::network_identity::production(){
            let device=payload["device_id"].as_str().and_then(|s|Uuid::parse_str(s).ok()).ok_or_else(||Self::boxed_error("Missing device identity".into()))?;
            format!("monitoring.{device}.{event_type}")
        }else{format!("monitoring.{event_type}")};
        let body = serde_json::to_vec(&payload)?;
        if body.len()>1048576{return Err(Self::boxed_error("Telemetry exceeds 1 MiB".into()));}
        let mut properties=BasicProperties::default().with_content_type("application/json".into()).with_delivery_mode(2);
        if crate::network_identity::production(){
            let url=reqwest::Url::parse(&self.rabbitmq_url)?;
            properties=properties.with_user_id(url.username().into());
        }

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

            let publish_result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                async {
                    let confirmation = channel
                        .basic_publish(
                            "monitoring",
                            &routing_key,
                            BasicPublishOptions { mandatory: true, ..Default::default() },
                            &body,
                            properties.clone(),
                        )
                        .await?
                        .await?;
                    match confirmation {
                        lapin::publisher_confirm::Confirmation::Ack(None) => Ok(()),
                        _ => Err(Self::boxed_error("Broker did not confirm a routed message".into())),
                    }
                }
            ).await;

            match publish_result {
                Ok(Ok(_)) => {
                    tracing::info!("✅ Event published successfully: {} ({} bytes)", routing_key, body.len());
                    return Ok(());
                }
                Ok(Err(e)) => {
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
                Err(_) => {
                    let err_msg = "Publish operation timed out after 5 seconds".to_string();
                    tracing::warn!(
                        "Publish attempt {} timed out for {}",
                        attempt,
                        routing_key
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
    pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut state = self.state.lock().await;
        if state.channel.is_none() {
            if Self::connect_locked(&self.rabbitmq_url, &mut state).await.is_err() {
                return Ok(false);
            }
        }

        let Some(channel) = state.channel.clone() else {
            return Ok(false);
        };

        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            channel.exchange_declare(
                "monitoring",
                lapin::ExchangeKind::Topic,
                lapin::options::ExchangeDeclareOptions {
                    durable: true,
                    passive: true,
                    ..Default::default()
                },
                Default::default(),
            )
        ).await {
            Ok(Ok(_)) => Ok(true),
            _ => {
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
    #[ignore = "requires PROBE_RABBITMQ_URL pointing to a disposable broker"]
    async fn broker_confirms_routed_messages_and_rejects_unroutable_messages() {
        let url = std::env::var("PROBE_RABBITMQ_URL").unwrap();
        let publisher = RabbitMQPublisher::connect(&url).await.unwrap();
        let connection = Connection::connect(&url, ConnectionProperties::default()).await.unwrap();
        let channel = connection.create_channel().await.unwrap();
        let queue = channel.queue_declare("", lapin::options::QueueDeclareOptions {
            exclusive: true, auto_delete: true, ..Default::default()
        }, Default::default()).await.unwrap();
        let event_type = format!("probe_{}", Uuid::new_v4().simple());
        let key = format!("monitoring.{event_type}");
        channel.queue_bind(queue.name().as_str(), "monitoring", &key, Default::default(), Default::default()).await.unwrap();
        let payload = json!({"test_id":Uuid::new_v4().to_string()});
        publisher.publish_event(&event_type, payload.clone()).await.unwrap();
        let received = channel.basic_get(queue.name().as_str(), lapin::options::BasicGetOptions { no_ack: true }).await.unwrap().unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&received.delivery.data).unwrap(), payload);
        channel.queue_delete(queue.name().as_str(), Default::default()).await.unwrap();
        assert!(publisher.publish_event(&event_type, payload).await.is_err());
    }
}
