use std::sync::Arc;
use std::collections::HashSet;
use tokio::time::Duration;
use chrono::Utc;
use crate::inventory::InventoryScanner;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let is_session_0 = is_running_in_session_0();
    if cfg!(windows) && !is_session_0 {
        tracing::info!("Skipping software inventory task (handled by service)");
        return tokio::spawn(std::future::pending::<()>());
    }

    tokio::spawn(async move {
        let mut known_inventory_fingerprints: HashSet<String> = HashSet::new();
        let mut interval = skip_interval(Duration::from_secs(60 * 60 * 24 * 30)); // Every 30 days

        // Initial baseline snapshot when agent starts.
        let initial_apps = match InventoryScanner::scan_installed_software().await {
            Ok(apps) => Some(apps),
            Err(e) => {
                tracing::warn!("Initial inventory scan error: {}", e);
                None
            }
        };

        if let Some(apps) = initial_apps {
            for app in &apps {
                let key = InventoryScanner::fingerprint(
                    &app.app_name,
                    app.version.as_deref(),
                    &app.exe_hash,
                );
                known_inventory_fingerprints.insert(key);
            }

            let detected_at = Utc::now().to_rfc3339();
            let inventory_payload = context.build_event_envelope(
                "inventory",
                1,
                serde_json::json!({
                    "detected_at": detected_at,
                    "apps": apps.into_iter().map(|app| serde_json::json!({
                        "app_name": app.app_name,
                        "version": app.version,
                        "exe_hash": app.exe_hash,
                        "detected_at": detected_at,
                    })).collect::<Vec<_>>(),
                }),
            );
            context.publish_or_cache("inventory", inventory_payload).await;
            tracing::info!("✅ Initial inventory snapshot published: {} apps", known_inventory_fingerprints.len());
        }

        loop {
            interval.tick().await;
            
            let apps = match InventoryScanner::scan_installed_software().await {
                Ok(apps) => Some(apps),
                Err(e) => {
                    tracing::warn!("Inventory scan error: {}", e);
                    None
                }
            };

            let Some(apps) = apps else {
                continue;
            };

            let mut new_apps = Vec::new();
            for app in apps {
                let key = InventoryScanner::fingerprint(
                    &app.app_name,
                    app.version.as_deref(),
                    &app.exe_hash,
                );

                if known_inventory_fingerprints.insert(key) {
                    new_apps.push(app);
                }
            }

            if new_apps.is_empty() {
                tracing::info!("Software inventory weekly scan complete: no new applications detected");
                continue;
            }

            tracing::info!(
                "Software inventory weekly scan complete: {} new apps detected",
                new_apps.len()
            );

            let detected_at = Utc::now().to_rfc3339();
            let inventory_payload = context.build_event_envelope(
                "inventory",
                1,
                serde_json::json!({
                    "detected_at": detected_at,
                    "apps": new_apps.into_iter().map(|app| serde_json::json!({
                        "app_name": app.app_name,
                        "version": app.version,
                        "exe_hash": app.exe_hash,
                        "detected_at": detected_at,
                    })).collect::<Vec<_>>(),
                }),
            );
            context.publish_or_cache("inventory", inventory_payload).await;
        }
    })
}
