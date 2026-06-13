use super::models::*;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::*;

pub async fn get_activity_logs(pool: &PgPool, device_id: Option<Uuid>) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let logs = if let Some(did) = device_id {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs WHERE device_id = $1 ORDER BY timestamp DESC"
            )
            .bind(did)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs ORDER BY timestamp DESC"
            )
            .fetch_all(pool)
            .await?
        };

        Ok(logs
            .into_iter()
            .map(|(id, device_id, app_name, window_title, duration_seconds, timestamp)| ActivityLog {
                id,
                device_id,
                app_name,
                window_title,
                duration_seconds,
                timestamp,
            })
            .collect())
    }

