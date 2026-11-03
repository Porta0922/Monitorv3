use std::sync::Arc;
use crate::is_running_in_session_0;
use super::{TaskContext, TaskInterval, live_sleep};

pub fn spawn(context: Arc<TaskContext>) -> tokio::task::JoinHandle<()> {
    let is_session_0 = is_running_in_session_0();
    if is_session_0 {
        tracing::info!("Skipping heatmap task (requires user session)");
        return tokio::spawn(std::future::pending::<()>());
    }

    tokio::spawn(async move {
        // Current native workers emit aggregate categories, not coordinates. Do not
        // publish an empty/fixed-resolution grid as an observed desktop heatmap.
        loop {
            crate::capture_diagnostics::record("heatmap","capture_error",Some("Coordinate capture is not available in this agent; aggregate input metrics remain available"));
            live_sleep(&context, TaskInterval::Heatmap).await;
        }
    })
}
