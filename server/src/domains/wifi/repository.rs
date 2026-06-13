use super::models::WifiEvent;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

pub async fn insert_wifi_event(
    pool: &PgPool,
    device_id: String,
    interface_name: String,
    state: String,
    ssid: Option<String>,
    bssid: Option<String>,
    signal_percent: Option<i32>,
    timestamp: DateTime<Utc>,
) -> Result<WifiEvent, sqlx::Error> {
    let id = Uuid::new_v4();
    let device_uuid = Uuid::parse_str(&device_id)
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

    sqlx::query(
        r#"
        INSERT INTO wifi_events (id, device_id, interface_name, state, ssid, bssid, signal_percent, timestamp)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(id)
    .bind(device_uuid)
    .bind(&interface_name)
    .bind(&state)
    .bind(&ssid)
    .bind(&bssid)
    .bind(signal_percent)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(WifiEvent {
        id,
        device_id: device_uuid,
        interface_name,
        state,
        ssid,
        bssid,
        signal_percent,
        timestamp,
    })
}

pub async fn get_wifi_events(
    pool: &PgPool,
    device_id: Option<Uuid>,
    limit: Option<i64>,
    date: Option<NaiveDate>,
    tz_offset_minutes: i32,
) -> Result<Vec<WifiEvent>, sqlx::Error> {
    let limit_value = limit.unwrap_or(1_000_000).max(1);
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, Option<String>, Option<String>, Option<i32>, DateTime<Utc>)>(
        "SELECT id, device_id, interface_name, state, ssid, bssid, signal_percent, timestamp
         FROM wifi_events
         WHERE ($1::uuid IS NULL OR device_id = $1)
           AND ($2::date IS NULL OR DATE(timestamp + ($3 * INTERVAL '1 minute')) = $2)
         ORDER BY timestamp DESC
         LIMIT $4"
    )
    .bind(device_id)
    .bind(date)
    .bind(tz_offset_minutes)
    .bind(limit_value)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, device_id, interface_name, state, ssid, bssid, signal_percent, timestamp)| WifiEvent {
            id,
            device_id,
            interface_name,
            state,
            ssid,
            bssid,
            signal_percent,
            timestamp,
        })
        .collect())
}
