use super::models::*;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::*;

pub async fn get_inventory(pool: &PgPool, device_id: Option<&str>) -> Result<Vec<InventoryItem>, sqlx::Error> {
        let items = if let Some(did) = device_id {
            let did_uuid = Uuid::parse_str(did)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT DISTINCT ON (app_name, COALESCE(version, ''), COALESCE(exe_hash, ''))
                        id::text, device_id::text, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
                 FROM inventory
                 WHERE device_id = $1
                 ORDER BY app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp DESC"
            )
            .bind(did_uuid)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT DISTINCT ON (device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''))
                        id::text, device_id::text, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
                 FROM inventory
                 ORDER BY device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp DESC"
            )
            .fetch_all(pool)
            .await?
        };

        Ok(items
            .into_iter()
            .map(|(id, device_id, app_name, version, exe_hash, timestamp)| InventoryItem {
                id,
                device_id,
                app_name,
                version,
                exe_hash,
                timestamp,
            })
            .collect())
    }

pub async fn get_running_apps(pool: &PgPool, device_id: Uuid) -> Result<Vec<RunningAppItem>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, i32, Option<String>, Option<String>, DateTime<Utc>)>(
            "SELECT id, device_id, app_name, primary_title, window_count, exe_path, exe_hash, updated_at
             FROM running_apps_current
             WHERE device_id = $1
             ORDER BY window_count DESC, updated_at DESC, app_name ASC"
        )
        .bind(device_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, device_id, app_name, primary_title, window_count, exe_path, exe_hash, updated_at)| RunningAppItem {
                id,
                device_id,
                app_name,
                primary_title: primary_title.unwrap_or_default(),
                window_count,
                exe_path,
                exe_hash,
                updated_at,
            })
            .collect())
    }

pub async fn get_top_apps(pool: &PgPool, days: i64) -> Result<Vec<TopApp>, sqlx::Error> {
        let apps = sqlx::query_as::<_, (String, i64)>(
            "SELECT app_name, COALESCE(SUM(duration_seconds), 0)::BIGINT as total_duration FROM activity_logs 
             WHERE timestamp >= NOW() - INTERVAL '1 day' * $1 
             GROUP BY app_name 
             ORDER BY total_duration DESC 
             LIMIT 6"
        )
        .bind(days)
        .fetch_all(pool)
        .await?;

        Ok(apps
            .into_iter()
            .map(|(app_name, total_duration_seconds)| TopApp {
                app_name,
                total_duration_seconds,
            })
            .collect())
    }

