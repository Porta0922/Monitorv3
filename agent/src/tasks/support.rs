use std::sync::Arc;
use tokio::time::Duration;
use chrono::Utc;
use super::TaskContext;

pub fn spawn_reconnector(context: Arc<TaskContext>, rabbitmq_url: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        use rand::Rng;
        let mut failures: u32 = 0;
        loop {
            // Clone outside the shared lock: network waits must not block collectors.
            let publisher = { context.publisher.read().await.clone() };
            let result = match publisher {
                Some(publisher) => publisher.ensure_connected().await.map_err(|e| e.to_string()),
                None => match crate::rabbitmq_publisher::RabbitMQPublisher::connect(
                    &crate::network_identity::rabbit_url(&rabbitmq_url),
                ).await {
                    Ok(publisher) => {
                        let mut slot = context.publisher.write().await;
                        if slot.is_none() { *slot = Some(Arc::new(publisher)); }
                        Ok(true)
                    }
                    Err(error) => Err(error.to_string()),
                },
            };
            let delay = match result {
                Ok(restored) => {
                    if restored {
                        tracing::info!("Connection watchdog restored RabbitMQ; pending events will resume");
                        context.wifi_resend_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    failures = 0;
                    Duration::from_secs(5)
                }
                Err(_) => {
                    failures = failures.saturating_add(1);
                    // Do not log broker credentials or authentication responses.
                    tracing::warn!(failures, "Connection watchdog could not restore RabbitMQ; outbox retained");
                    let base = (2u64.saturating_pow(failures.min(6))).min(60) as f64;
                    let jitter = rand::thread_rng().gen_range(0.85..=1.15);
                    Duration::from_secs_f64((base * jitter).clamp(1.0, 60.0))
                }
            };
            tokio::time::sleep(delay).await;
        }
    })
}


pub fn spawn_shutdown_listener(context: Arc<TaskContext>) {
    let context_clone = context.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let now = Utc::now();
            let alert_payload = context_clone.build_event_envelope(
                "security",
                1,
                serde_json::json!({
                    "alert_type": "PROCESS_TERMINATION_ATTEMPTED",
                    "severity": "CRITICAL",
                    "description": "Signal de terminacion recibida por el agent",
                    "method": "signal.ctrl_c",
                    "attempted_by": serde_json::Value::Null,
                    "blocked": false,
                    "auto_restarted": false,
                    "timestamp": now.to_rfc3339(),
                }),
            );

            context_clone.publish_or_cache("security", alert_payload).await;
            tracing::warn!("Termination signal detected. Security alert emitted.");
        }
    });
}

pub fn spawn_retry_synchronizer(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        use crate::outbox_drain::{drain_batch, DrainStatus};
        let idle_delay = Duration::from_secs(20);
        let mut delay = Duration::ZERO;
        let mut next_cleanup = tokio::time::Instant::now() + Duration::from_secs(300);
        loop {
            tokio::time::sleep(delay).await;
            delay = idle_delay;
            let publisher = { context.publisher.read().await.clone() };
            if let Some(publisher) = publisher {
                match drain_batch(&context.cache, |route, payload| {
                    let publisher = publisher.clone();
                    let context = context.clone();
                    async move {
                        let mut payload = payload;
                        crate::privacy::apply(&mut payload, &context.config_manager.get().await);
                        publisher.publish_event(&route, payload).await.map_err(|e| e.to_string())
                    }
                }).await {
                    Ok(DrainStatus::More) => delay = Duration::from_millis(250),
                    Ok(DrainStatus::Idle | DrainStatus::Interrupted) => {},
                    Err(error) => tracing::error!(%error, "Outbox replay storage failure; pending data retained"),
                }
            }
            if tokio::time::Instant::now() >= next_cleanup {
                if let Err(error) = context.cache.cleanup_synced(3).await {
                    tracing::warn!(%error, "Confirmed outbox cleanup failed");
                }
                next_cleanup = tokio::time::Instant::now() + Duration::from_secs(300);
            }
        }
    })
}
