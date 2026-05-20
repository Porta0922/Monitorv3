use std::sync::Arc;
use tokio::time::Duration;
use chrono::Utc;
use super::TaskContext;

pub fn spawn_reconnector(context: Arc<TaskContext>, rabbitmq_url: String) {
    let publisher_reconnect = context.publisher.clone();
    let wifi_resend_flag_reconnect = context.wifi_resend_flag.clone();
    tokio::spawn(async move {
        use rand::Rng;
        let mut retry_count = 0;
        let base_delay = 2.0; // 2 segundos base
        let max_delay = 300.0; // máximo 5 minutos (300 segundos)

        loop {
            let needs_connect = {
                let state = publisher_reconnect.read().await;
                state.is_none()
            };

            if !needs_connect {
                retry_count = 0; // Reset retry count if connected
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            // Calcular el retraso exponencial
            let exp_delay = (base_delay * 2f64.powi(retry_count)).min(max_delay);
            
            // Jitter: añadir fluctuación aleatoria de ±15% (factor entre 0.85 y 1.15)
            let final_delay = {
                let mut rng = rand::thread_rng();
                let factor: f64 = rng.gen_range(0.85..=1.15);
                (exp_delay * factor).max(1.0).min(max_delay)
            };

            tracing::debug!("Intento de reconexión a RabbitMQ #{} en {:.2} segundos...", retry_count + 1, final_delay);
            tokio::time::sleep(Duration::from_secs_f64(final_delay)).await;

            // Re-verificar si seguimos requiriendo la conexión justo antes del intento
            let needs_connect = {
                let state = publisher_reconnect.read().await;
                state.is_none()
            };

            if !needs_connect {
                continue;
            }

            let reconnect_result = crate::rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url)
                .await
                .map(Arc::new)
                .map_err(|e| e.to_string());

            match reconnect_result {
                Ok(conn) => {
                    let mut state = publisher_reconnect.write().await;
                    if state.is_none() {
                        *state = Some(conn);
                        tracing::info!("✅ RabbitMQ reconnected successfully");
                        wifi_resend_flag_reconnect.store(true, std::sync::atomic::Ordering::Relaxed);
                        retry_count = 0; // Reset retry count upon success
                    }
                }
                Err(e) => {
                    tracing::warn!("RabbitMQ still unavailable during reconnect attempt: {}", e);
                    retry_count += 1;
                }
            }
        }
    });
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

pub fn spawn_retry_synchronizer(context: Arc<TaskContext>) {
    let publisher_clone = context.publisher.clone();
    let cache_clone = context.cache.clone();
    tokio::spawn(async move {
        let mut retry_interval = super::skip_interval(Duration::from_secs(20));
        loop {
            retry_interval.tick().await;

            let publisher_snapshot = { publisher_clone.read().await.clone() };
            let Some(pub_) = publisher_snapshot else {
                continue;
            };

            match cache_clone.get_unsynced_events().await {
                Ok(events) if !events.is_empty() => {
                    let mut synced_ids = Vec::new();

                    for (event_id, event_type, payload) in events {
                        match pub_.publish_event(&event_type, payload).await {
                            Ok(_) => synced_ids.push(event_id),
                            Err(e) => {
                                tracing::warn!("Retry publish failed for {}: {}", event_type, e);
                                break;
                            }
                        }
                    }

                    if !synced_ids.is_empty() {
                        let _ = cache_clone.mark_synced(&synced_ids).await;
                    }

                      let _ = cache_clone.cleanup_synced(3).await;
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("Failed reading offline cache: {}", e),
            }
        }
    });
}
