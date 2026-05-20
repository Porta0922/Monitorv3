use std::sync::Arc;
use tokio::time::{Duration, Instant};
use chrono::Utc;
use crate::monitoring::MonitoringLoop;
use crate::is_running_in_session_0;
use super::{TaskContext, skip_interval};

fn is_unknown_like(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.is_empty()
        || normalized == "unknown"
        || normalized == "n/a"
        || normalized == "<unknown>"
        || normalized == "(unknown)"
}

fn sanitize_activity_fields(app_name: &str, window_title: &str) -> (String, String) {
    let clean_window = if is_unknown_like(window_title) {
        "Sin titulo".to_string()
    } else {
        window_title.trim().to_string()
    };

    let clean_app = if is_unknown_like(app_name) {
        if clean_window != "Sin titulo" {
            clean_window.clone()
        } else {
            "Sin identificar".to_string()
        }
    } else {
        app_name.trim().to_string()
    };

    (clean_app, clean_window)
}

pub fn spawn(context: Arc<TaskContext>) {
    tokio::spawn(async move {
        let monitoring = MonitoringLoop::new();
        let mut interval = skip_interval(Duration::from_secs(2));
        let mut last_window: Option<(String, String, Instant)> = None;
        let mut last_report_instant: Option<Instant> = None;
        let mut last_loop_time = Instant::now();

        loop {
            interval.tick().await;
            
            // Session 0 (Windows Service or Unix root service) cannot see user windows or capture input.
            // We skip these monitors if running as a system service.
            if is_running_in_session_0() {
                continue;
            }

            let now_instant = Instant::now();
            let time_since_last_loop = now_instant.duration_since(last_loop_time);

            if time_since_last_loop > Duration::from_secs(10) {
                tracing::info!(
                    "⏰ System suspension detected. Elapsed since last loop: {}s. Resetting activity duration.",
                    time_since_last_loop.as_secs()
                );

                // Finalize the last window using the last_loop_time (before suspension)
                if let Some((last_app, last_title, _)) = last_window.take() {
                    if let Some(report_ref) = last_report_instant {
                        if last_loop_time > report_ref {
                            let duration_seconds = last_loop_time.duration_since(report_ref).as_secs();
                            if duration_seconds > 0 {
                                let activity_payload = context.build_event_envelope(
                                    "activity",
                                    1,
                                    serde_json::json!({
                                        "app_name": last_app,
                                        "window_title": last_title,
                                        "duration_seconds": duration_seconds,
                                        "timestamp": Utc::now().to_rfc3339(),
                                    }),
                                );
                                context.publish_or_cache("activity", activity_payload).await;
                                tracing::info!(
                                    "[ACTIVITY] Finalized before sleep: {} (Duration: {}s)",
                                    last_app,
                                    duration_seconds
                                );
                            }
                        }
                    }
                }
                
                last_window = None;
                last_report_instant = None;
            }

            last_loop_time = now_instant;

            // Update idle status based on recent activity
            context.keystroke_tracker.update_idle_status().await;
            let stats = context.keystroke_tracker.get_stats().await;
            
            if stats.is_idle {
                // If user is idle, we "pause" activity reporting.
                // We also clear last_window so that when they return, it starts a new session.
                if let Some((last_app, last_title, _started_at)) = last_window.take() {
                    let now_utc = Utc::now();
                    let now_instant = Instant::now();
                    let duration_reference = last_report_instant.unwrap_or(now_instant);
                    let duration_seconds = now_instant.duration_since(duration_reference).as_secs();
                    
                    let activity_payload = context.build_event_envelope(
                        "activity",
                        1,
                        serde_json::json!({
                            "app_name": last_app,
                            "window_title": last_title,
                            "duration_seconds": duration_seconds,
                            "timestamp": now_utc.to_rfc3339(),
                        }),
                    );
                    context.publish_or_cache("activity", activity_payload).await;
                }
                last_report_instant = None;
                continue;
            }
            
            if let Some(current) = monitoring.capture_active_window() {
                let (current_app, current_title) = sanitize_activity_fields(&current.app_name, &current.window_title);

                if let Some((last_app, last_title, _started_at)) = &last_window {
                    let changed = *last_app != current_app || *last_title != current_title;
                    
                    if changed {
                        // Window changed: send activity event for previous window
                        let now_instant = Instant::now();
                        let duration_reference = last_report_instant.unwrap_or(now_instant);
                        let duration_seconds = now_instant.duration_since(duration_reference).as_secs();
                        
                        // Only send if we have meaningful duration
                        if duration_seconds > 0 {
                            let activity_payload = context.build_event_envelope(
                                "activity",
                                1,
                                serde_json::json!({
                                    "app_name": last_app,
                                    "window_title": last_title,
                                    "duration_seconds": duration_seconds,
                                    "timestamp": current.timestamp.to_rfc3339(),
                                }),
                            );

                            context.publish_or_cache("activity", activity_payload).await;
                            tracing::info!("[ACTIVITY] Window change: {} -> {} (Duration: {}s)", last_app, current_app, duration_seconds);
                        }

                        last_window = Some((current_app.clone(), current_title.clone(), now_instant));
                        last_report_instant = Some(now_instant);
                    } else {
                        // Window unchanged: send activity heartbeat every 30 seconds to show continuation
                        let now_utc = Utc::now();
                        let now_instant = Instant::now();
                        let elapsed = last_report_instant.map(|l| now_instant.duration_since(l).as_secs()).unwrap_or(0);
                        
                        if elapsed >= 30 {
                            let duration_reference = last_report_instant.unwrap_or(now_instant);
                            let duration_seconds = now_instant.duration_since(duration_reference).as_secs();
                            
                            if duration_seconds > 0 {
                                let activity_payload = context.build_event_envelope(
                                    "activity",
                                    1,
                                    serde_json::json!({
                                        "app_name": last_app,
                                        "window_title": last_title,
                                        "duration_seconds": duration_seconds,
                                        "timestamp": now_utc.to_rfc3339(),
                                    }),
                                );

                                context.publish_or_cache("activity", activity_payload).await;
                                tracing::info!("[ACTIVITY] Heartbeat: {} (Duration: {}s)", last_app, duration_seconds);
                            }
                            last_report_instant = Some(now_instant);
                        }
                    }
                } else {
                    // First window captured
                    let now_instant = Instant::now();
                    last_window = Some((current_app, current_title, now_instant));
                    last_report_instant = Some(now_instant);
                }
            } else {
                // No window in focus: update last_report_instant so we don't accumulate this time
                // to the next window that gets focus.
                let now_instant = Instant::now();
                if last_window.is_some() {
                    // If we had a window, we should finalize it before clearing
                    if let Some((last_app, last_title, _)) = last_window.take() {
                        let duration_reference = last_report_instant.unwrap_or(now_instant);
                        let duration_seconds = now_instant.duration_since(duration_reference).as_secs();
                        
                        if duration_seconds > 0 {
                            let activity_payload = context.build_event_envelope(
                                "activity",
                                1,
                                serde_json::json!({
                                    "app_name": last_app,
                                    "window_title": last_title,
                                    "duration_seconds": duration_seconds,
                                    "timestamp": Utc::now().to_rfc3339(),
                                }),
                            );
                            context.publish_or_cache("activity", activity_payload).await;
                            tracing::info!("[ACTIVITY] Focus lost: finalized {} (Duration: {}s)", last_app, duration_seconds);
                        }
                    }
                }
                last_report_instant = Some(now_instant);
            }
        }
    });
}
