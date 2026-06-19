use std::sync::Arc;
use tokio::time::Duration;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let is_session_0 = is_running_in_session_0();
    if is_session_0 {
        tracing::info!("Skipping heatmap task (requires user session)");
        return tokio::spawn(std::future::pending::<()>());
    }

    tokio::spawn(async move {
        let mut interval = skip_interval(Duration::from_secs(3600));  // Every hour
        loop {
            interval.tick().await;
            
            // Check if heatmap should be uploaded
            if context.input_tracker.should_upload().await {
                if let Some(heatmap) = context.input_tracker.get_heatmap_for_upload().await {
                    tracing::debug!(
                        "📊 Heatmap ready for upload: {} mouse moves, {} keyboard events",
                        heatmap.stats.mouse_moves,
                        heatmap.stats.keyboard_events
                    );
                    
                    let event = context.build_event_envelope(
                        "input_heatmap",
                        1,
                        serde_json::json!({
                            "heatmap": heatmap,
                        }),
                    );

                    context.publish_or_cache("input_heatmaps", event).await;
                }
            }
        }
    })
}
