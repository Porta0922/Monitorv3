use std::future::Future;
use std::time::Duration;
use serde_json::Value;
use crate::offline_cache::OfflineCache;

pub enum DrainStatus { Idle, More, Interrupted }

/// Bound each pass by both the cache batch size and wall time. A cancelled publish
/// may have reached the broker: retain its stable ID for deduplicated replay.
pub async fn drain_batch<F, Fut>(cache: &OfflineCache, publish: F) -> rusqlite::Result<DrainStatus>
where F: FnMut(String, Value) -> Fut, Fut: Future<Output = Result<(), String>> {
    drain_with_budget(cache, publish, Duration::from_secs(5)).await
}

async fn drain_with_budget<F, Fut>(cache: &OfflineCache, mut publish: F, budget: Duration) -> rusqlite::Result<DrainStatus>
where F: FnMut(String, Value) -> Fut, Fut: Future<Output = Result<(), String>> {
    let events = cache.get_unsynced_events().await?;
    let full_batch = events.len() == crate::offline_cache::REPLAY_BATCH_SIZE;
    let deadline = tokio::time::Instant::now() + budget;
    let mut confirmed = Vec::with_capacity(events.len());
    let mut interrupted = false;
    let mut budget_exhausted = false;
    for (id, route, payload) in events {
        if tokio::time::Instant::now() >= deadline { budget_exhausted = true; break; }
        match tokio::time::timeout_at(deadline, publish(route, payload)).await {
            Ok(Ok(())) => confirmed.push(id),
            Ok(Err(error)) => {
                tracing::warn!(%error, "Outbox replay interrupted; unconfirmed events retained");
                interrupted = true;
                break;
            }
            Err(_) => {
                tracing::warn!("Outbox replay budget reached; uncertain publication retained");
                budget_exhausted = true;
                break;
            }
        }
    }
    // A failed local commit retains the whole batch for at-least-once delivery.
    if !confirmed.is_empty() { cache.mark_synced(&confirmed).await?; }
    Ok(if interrupted || (budget_exhausted && confirmed.is_empty()) { DrainStatus::Interrupted }
        else if full_batch || budget_exhausted { DrainStatus::More } else { DrainStatus::Idle })
}

#[cfg(test)]
mod durability_tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn drains_large_backlog_in_bounded_batches_without_changing_ids() {
        let cache = OfflineCache::new(":memory:", &[9; 32]).unwrap();
        let mut ids = Vec::new();
        for sequence in 0..5000 {
            let id = uuid::Uuid::new_v4().to_string();
            cache.save_event("heartbeat", &json!({"event_id":id,"sequence":sequence})).await.unwrap();
            ids.push(id);
        }
        let mut received = Vec::new();
        let mut passes = 0;
        loop {
            let before = received.len();
            let status = drain_batch(&cache, |_, payload| {
                received.push(payload["event_id"].as_str().unwrap().to_string());
                std::future::ready(Ok(()))
            }).await.unwrap();
            assert!(received.len() - before <= crate::offline_cache::REPLAY_BATCH_SIZE);
            passes += 1;
            if !matches!(status, DrainStatus::More) { break; }
            assert!(passes < 25);
        }
        assert_eq!(received, ids);
        assert_eq!(cache.get_unsynced_event_count().await.unwrap(), 0);
        assert_eq!(passes, 20);
    }

    #[tokio::test]
    async fn failed_and_timed_out_publications_remain_pending() {
        let cache = OfflineCache::new(":memory:", &[9; 32]).unwrap();
        for _ in 0..5 { cache.save_event("heartbeat", &json!({"event_id":uuid::Uuid::new_v4()})).await.unwrap(); }
        let mut attempts = 0;
        let status = drain_batch(&cache, |_, _| {
            attempts += 1;
            std::future::ready(if attempts == 3 { Err("disconnected".into()) } else { Ok(()) })
        }).await.unwrap();
        assert!(matches!(status, DrainStatus::Interrupted));
        assert_eq!(cache.get_unsynced_event_count().await.unwrap(), 3);
        let mut attempts = 0;
        let status = drain_with_budget(&cache, |_, _| {
            attempts += 1;
            let attempt = attempts;
            async move { if attempt == 1 { Ok(()) } else { std::future::pending().await } }
        }, Duration::from_millis(5)).await.unwrap();
        assert!(matches!(status, DrainStatus::More));
        assert_eq!(cache.get_unsynced_event_count().await.unwrap(), 2);
        let status = drain_with_budget(&cache, |_, _| std::future::pending(), Duration::from_millis(5)).await.unwrap();
        assert!(matches!(status, DrainStatus::Interrupted));
        assert_eq!(cache.get_unsynced_event_count().await.unwrap(), 2);
    }
}
