//! Authoritative service snapshots and dynamic session collector lifecycle.
use std::{io, sync::Arc, time::Duration};
use crate::{session_relay::SessionRelay, tasks::TaskContext};

pub async fn bootstrap(context: &TaskContext, relay: &SessionRelay) -> io::Result<()> {
    match relay.policy(&context.device_id).await {
        Ok(policy) => context.config_manager.apply_policy(policy).await?,
        Err(error) => {
            if !crate::runtime::data_dir().join("effective_policy.json").is_file() { return Err(error); }
            tracing::warn!(%error, "Service policy unavailable; using previously validated snapshot");
        }
    }
    context.keystroke_tracker.set_idle_threshold(context.config_manager.get().await.idle_threshold_seconds);
    Ok(())
}

pub fn spawn_sync(context: Arc<TaskContext>, relay: Arc<SessionRelay>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut timer = crate::tasks::skip_interval(Duration::from_secs(15));
        loop {
            timer.tick().await;
            match relay.policy(&context.device_id).await {
                Ok(policy) => {
                    let threshold = policy.idle_threshold_seconds;
                    match context.config_manager.apply_policy(policy).await {
                        Ok(()) => context.keystroke_tracker.set_idle_threshold(threshold),
                        Err(error) => tracing::error!(%error, "Session policy could not be persisted; previous snapshot retained"),
                    }
                }
                Err(error) => tracing::warn!(%error, "Policy refresh deferred; previous snapshot retained"),
            }
        }
    })
}

struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop { fn drop(&mut self) { self.0.abort(); } }

pub fn spawn_collector(context: Arc<TaskContext>, name: &'static str) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut changes = context.config_manager.subscribe();
        loop {
            let enabled = changes.borrow_and_update().is_monitor_enabled(name) && crate::runtime::permits_task(name);
            if !enabled {
                if name=="session_input" && changes.borrow().is_monitor_enabled(name) {
                    let diagnostic=crate::runtime::input_diagnostic();
                    crate::capture_diagnostics::record("session_input","capture_error",Some(diagnostic["last_error"].as_str().unwrap_or("Input adapter has not confirmed availability")));
                }
                // Native input can become available after the policy was loaded (e.g. X11 first event).
                // Observe capability recovery even when no policy revision arrives.
                tokio::select! {
                    changed = changes.changed() => { if changed.is_err() { return; } }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                }
                continue;
            }
            if name == "session_input" { context.keystroke_tracker.reset_minute_counters().await; }
            if name == "heatmap" { context.input_tracker.discard_current_period().await; }
            let mut worker = match name {
                "window_activity" => crate::tasks::window_activity::spawn(context.clone()),
                "running_apps" => crate::tasks::running_apps::spawn(context.clone()),
                "heatmap" => crate::tasks::heatmap::spawn(context.clone()),
                "session_input" => crate::tasks::session_input::spawn(context.clone()),
                _ => return,
            };
            let guard = AbortOnDrop(worker.abort_handle());
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        if !crate::runtime::permits_task(name) {
                            worker.abort(); let _ = worker.await;
                            break;
                        }
                    }
                    result = &mut worker => {
                        tracing::warn!(monitor = name, ?result, "Session collector stopped; supervisor will restart wrapper");
                        return;
                    }
                    changed = changes.changed() => {
                        if changed.is_err() { return; }
                        let still_enabled = changes.borrow_and_update().is_monitor_enabled(name);
                        if !still_enabled {
                            worker.abort(); let _ = worker.await;
                            break;
                        }
                    }
                }
            }
            drop(guard);
        }
    })
}

/// Guard existing service collectors too, so a disabled-at-start task can be enabled later.
pub fn spawn_service_collector<F>(config: Arc<crate::config_manager::ConfigManager>, name: String,
    factory: Arc<std::sync::Mutex<F>>) -> tokio::task::JoinHandle<()>
where F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static {
    tokio::spawn(async move {
        let mut changes = config.subscribe();
        loop {
            let enabled = changes.borrow_and_update().is_monitor_enabled(&name);
            if !enabled {
                if changes.changed().await.is_err() { return; }
                continue;
            }
            let mut worker = { let factory = factory.lock().unwrap(); factory() };
            let guard = AbortOnDrop(worker.abort_handle());
            loop {
                tokio::select! {
                    result = &mut worker => {
                        tracing::warn!(monitor = %name, ?result, "Collector stopped; supervisor will restart wrapper");
                        return;
                    }
                    changed = changes.changed() => {
                        if changed.is_err() { return; }
                        let still_enabled = changes.borrow_and_update().is_monitor_enabled(&name);
                        if !still_enabled {
                            worker.abort(); let _ = worker.await;
                            break;
                        }
                    }
                }
            }
            drop(guard);
        }
    })
}

#[cfg(test)]
mod dynamic_policy_tests {
    use super::*;
    use crate::config_manager::{AgentConfig, ConfigManager};
    #[tokio::test]
    async fn initially_disabled_collector_can_start_pause_resume_and_stop() {
        let dir = std::env::temp_dir().join(format!("monitor-dynamic-{}", uuid::Uuid::new_v4()));
        let config = ConfigManager::from_path(dir.join("policy.json")).unwrap();
        let mut policy = AgentConfig::default(); policy.disabled_monitors = vec!["heartbeat".into()];
        config.apply_policy(policy).await.unwrap();
        let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stopped_tx, mut stopped_rx) = tokio::sync::mpsc::unbounded_channel();
        struct StopNotice(tokio::sync::mpsc::UnboundedSender<()>);
        impl Drop for StopNotice { fn drop(&mut self) { let _ = self.0.send(()); } }
        let factory = Arc::new(std::sync::Mutex::new(move || {
            let started = started_tx.clone(); let stopped = stopped_tx.clone();
            tokio::spawn(async move {
                let _notice = StopNotice(stopped);
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            })
        }));
        let wrapper = spawn_service_collector(config.clone(), "heartbeat".into(), factory);
        tokio::task::yield_now().await;
        assert!(started_rx.try_recv().is_err());
        config.apply_policy(AgentConfig::default()).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), started_rx.recv()).await.unwrap().unwrap();
        let mut paused = AgentConfig::default(); paused.monitoring_paused = true;
        config.apply_policy(paused).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), stopped_rx.recv()).await.unwrap().unwrap();
        config.apply_policy(AgentConfig::default()).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), started_rx.recv()).await.unwrap().unwrap();
        wrapper.abort(); let _ = wrapper.await;
        tokio::time::timeout(Duration::from_secs(2), stopped_rx.recv()).await.unwrap().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
