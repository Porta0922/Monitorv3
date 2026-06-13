use super::models::*;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::*;

pub async fn insert_usb_event(
        pool: &PgPool,
        device_id: String,
        action: String,
        hardware_id: String,
        device_name: String,
        serial_number: Option<String>,
        volume_label: Option<String>,
    ) -> Result<UsbEvent, sqlx::Error> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        sqlx::query(
            r#"
            INSERT INTO usb_events (id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(device_uuid)
        .bind(&action)
        .bind(&hardware_id)
        .bind(&device_name)
        .bind(&serial_number)
        .bind(&volume_label)
        .bind(timestamp)
        .execute(pool)
        .await?;

        Ok(UsbEvent {
            id,
            device_id: device_uuid,
            action,
            hardware_id,
            device_name,
            serial_number: serial_number.unwrap_or_default(),
            volume_label,
            timestamp,
        })
    }

pub async fn get_usb_events(
        pool: &PgPool,
        device_id: Option<Uuid>,
        limit: Option<i64>,
        date: Option<NaiveDate>,
        tz_offset_minutes: i32,
    ) -> Result<Vec<UsbEvent>, sqlx::Error> {
        let limit_value = limit.unwrap_or(1_000_000).max(1);
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, Option<String>, DateTime<Utc>)>(
            "SELECT id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp
             FROM usb_events
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
            .map(|(id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp)| UsbEvent {
                id,
                device_id,
                action,
                hardware_id,
                device_name,
                serial_number: serial_number.unwrap_or_default(),
                volume_label,
                timestamp,
            })
            .collect())
    }

