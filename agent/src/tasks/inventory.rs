use std::sync::Arc;
use super::{TaskContext, TaskInterval, live_sleep};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let mut failed=false;
            let mut capture_failed=false;
            let mut sources=0;
            for mut snapshot in crate::inventory_snapshot::scan().await {
                // Every snapshot is complete for one explicit source or reports failure.
                // The durable outbox owns retries; no in-memory fingerprint can hide a lost scan.
                sources += 1;
                capture_failed |= snapshot["complete"] != true;
                failed |= snapshot["complete"] != true;
                snapshot["interval_days"] = serde_json::json!(context.config_manager.get().await.inventory_interval_days);
                let event = context.build_event_envelope("inventory_snapshot", 2, snapshot);
                if !context.publish_or_cache("inventory", event).await {
                    failed=true;
                    tracing::warn!("Inventory snapshot was not accepted; next scan will reconcile state");
                }
            }
            crate::capture_diagnostics::record("inventory", if capture_failed { "capture_error" } else if sources == 0 { "no_observation" } else { "capture_ok" }, if capture_failed { Some("Una o más fuentes no completaron el escaneo") } else { None });
            if failed { tokio::time::sleep(std::time::Duration::from_secs(300)).await; }
            else { live_sleep(&context, TaskInterval::Inventory).await; }
        }
    })
}
