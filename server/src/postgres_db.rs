// PostgreSQL database module for real data storage
use sqlx::{PgPool, Postgres, QueryBuilder, postgres::PgPoolOptions};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: Uuid,
    pub device_id: Uuid,
    pub app_name: String,
    pub window_title: String,
    pub duration_seconds: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub device_id: String,
    pub app_name: String,
    pub version: String,
    pub exe_hash: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub action: String,
    pub hardware_id: String,
    pub device_name: String,
    pub serial_number: String,
    pub volume_label: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTimeTotals {
    pub device_id: Uuid,
    pub active_seconds: i64,
    pub idle_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveDeviceActivity {
    pub device_id: Uuid,
    pub app_name: String,
    pub window_title: String,
    pub duration_seconds: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Uuid,
    pub hostname: String,
    pub device_id: Uuid,
    pub mac_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub devices_today: i64,
    pub active_time: i64,
    pub idle_time: i64,
    pub idle_pct: f64,
    pub keys_today: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopApp {
    pub app_name: String,
    pub total_duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub device_id: String,
    pub app: String,
    pub title: String,
    pub is_idle: bool,
    pub is_live: bool,
    pub last_seen: String,
}

pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create connection pool to PostgreSQL
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        Self::init_schema(&pool).await?;

        Ok(Self { pool })
    }

    /// Initialize database schema if not exists
    async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
        // Create devices table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
                device_id UUID PRIMARY KEY,
                hostname VARCHAR(255) NOT NULL,
                nickname VARCHAR(255),
                mac_address VARCHAR(17),
                last_seen TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Reconcile older device table versions with the current schema.
        sqlx::query("ALTER TABLE devices ADD COLUMN IF NOT EXISTS nickname VARCHAR(255)")
            .execute(pool)
            .await?;
        sqlx::query("ALTER TABLE devices ADD COLUMN IF NOT EXISTS mac_address VARCHAR(17)")
            .execute(pool)
            .await?;
        sqlx::query("ALTER TABLE devices ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()")
            .execute(pool)
            .await?;
        sqlx::query("ALTER TABLE devices ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()")
            .execute(pool)
            .await?;
        sqlx::query("ALTER TABLE devices ADD COLUMN IF NOT EXISTS last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()")
            .execute(pool)
            .await?;

        // Create activity_logs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS activity_logs (
                id UUID PRIMARY KEY,
                device_id VARCHAR(255) NOT NULL REFERENCES devices(device_id),
                app_name VARCHAR(255) NOT NULL,
                window_title TEXT,
                duration_seconds BIGINT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create inventory table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS inventory (
                id UUID PRIMARY KEY,
                device_id VARCHAR(255) NOT NULL REFERENCES devices(device_id),
                app_name VARCHAR(255) NOT NULL,
                version VARCHAR(255),
                exe_hash VARCHAR(255),
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create USB events table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usb_events (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                action VARCHAR(10) NOT NULL,
                hardware_id VARCHAR(255) NOT NULL,
                device_name VARCHAR(255) NOT NULL,
                serial_number VARCHAR(255),
                volume_label VARCHAR(255),
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_usb_events_device_timestamp ON usb_events(device_id, timestamp DESC)"
        )
        .execute(pool)
        .await?;

        // Create input activity metrics table (minute-level active/idle summaries)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS input_activity_metrics (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                timestamp TIMESTAMPTZ NOT NULL,
                active_seconds BIGINT NOT NULL DEFAULT 0,
                idle_seconds BIGINT NOT NULL DEFAULT 0,
                status VARCHAR(16) NOT NULL DEFAULT 'active',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_input_activity_metrics_device_timestamp ON input_activity_metrics(device_id, timestamp DESC)"
        )
        .execute(pool)
        .await?;

        // Create processed events table for idempotency (dedupe retries)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS processed_events (
                event_id VARCHAR(64) PRIMARY KEY,
                device_id VARCHAR(255) NOT NULL,
                event_type VARCHAR(64) NOT NULL,
                sequence BIGINT,
                boot_id VARCHAR(64),
                received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_processed_events_device_received_at ON processed_events(device_id, received_at DESC)"
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn register_processed_event(
        &self,
        event_id: &str,
        device_id: &str,
        event_type: &str,
        sequence: Option<i64>,
        boot_id: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO processed_events (event_id, device_id, event_type, sequence, boot_id)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(event_id)
        .bind(device_id)
        .bind(event_type)
        .bind(sequence)
        .bind(boot_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // Activity Methods
    pub async fn insert_activity_log(
        &self,
        device_id: String,
        app_name: String,
        window_title: String,
        duration_seconds: i64,
    ) -> Result<ActivityLog, sqlx::Error> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        sqlx::query(
            r#"
            INSERT INTO activity_logs (id, device_id, app_name, window_title, duration_seconds, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(device_uuid)
        .bind(&app_name)
        .bind(&window_title)
        .bind(duration_seconds)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        tracing::info!("📝 Saved activity log: {} | {} | {} seconds", device_id, app_name, duration_seconds);

        Ok(ActivityLog {
            id,
            device_id: device_uuid,
            app_name,
            window_title,
            duration_seconds,
            timestamp,
        })
    }

    pub async fn get_activity_logs(&self, device_id: Option<Uuid>) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let logs = if let Some(did) = device_id {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs WHERE device_id = $1 ORDER BY timestamp DESC"
            )
            .bind(did)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs ORDER BY timestamp DESC"
            )
            .fetch_all(&self.pool)
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

    pub async fn get_activity_logs_filtered(
        &self,
        device_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let mut query_builder = QueryBuilder::<Postgres>::new(
            "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs"
        );

        let mut has_where = false;

        if let Some(did) = device_id {
            query_builder.push(" WHERE device_id = ").push_bind(did);
            has_where = true;
        }

        if let Some(from_ts) = from {
            if has_where {
                query_builder.push(" AND timestamp >= ");
            } else {
                query_builder.push(" WHERE timestamp >= ");
                has_where = true;
            }
            query_builder.push_bind(from_ts);
        }

        if let Some(to_ts) = to {
            if has_where {
                query_builder.push(" AND timestamp <= ");
            } else {
                query_builder.push(" WHERE timestamp <= ");
                has_where = true;
            }
            query_builder.push_bind(to_ts);
        }

        query_builder.push(" ORDER BY timestamp DESC");

        if let Some(limit_value) = limit {
            query_builder.push(" LIMIT ").push_bind(limit_value.max(1));
        }

        let rows = query_builder
            .build_query_as::<(Uuid, Uuid, String, String, i64, DateTime<Utc>)>()
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
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

    pub async fn get_device_history_for_date(
        &self,
        device_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<(String, String, i64, i64)>, sqlx::Error> {
        sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT app_name, window_title, COALESCE(SUM(duration_seconds), 0)::BIGINT, COUNT(*)::BIGINT
             FROM activity_logs
             WHERE device_id = $1 AND DATE(timestamp) = $2
             GROUP BY app_name, window_title
             ORDER BY 3 DESC"
        )
        .bind(device_id)
        .bind(date)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_device_hourly_for_date(
        &self,
        device_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<(i32, i64, i64)>, sqlx::Error> {
        sqlx::query_as::<_, (i32, i64, i64)>(
            "SELECT EXTRACT(HOUR FROM timestamp)::INT,
                    COALESCE(SUM(CASE WHEN LOWER(app_name) LIKE '%idle%' OR LOWER(window_title) LIKE '%idle%' THEN 0 ELSE duration_seconds END), 0)::BIGINT,
                    COALESCE(SUM(CASE WHEN LOWER(app_name) LIKE '%idle%' OR LOWER(window_title) LIKE '%idle%' THEN duration_seconds ELSE 0 END), 0)::BIGINT
             FROM activity_logs
             WHERE device_id = $1 AND DATE(timestamp) = $2
             GROUP BY 1
             ORDER BY 1"
        )
        .bind(device_id)
        .bind(date)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_available_dates(
        &self,
        device_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<NaiveDate>, sqlx::Error> {
        if let Some(did) = device_id {
            sqlx::query_scalar::<_, NaiveDate>(
                "SELECT DISTINCT DATE(timestamp)
                 FROM activity_logs
                 WHERE device_id = $1
                 ORDER BY 1 DESC
                 LIMIT $2"
            )
            .bind(did)
            .bind(limit.max(1))
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_scalar::<_, NaiveDate>(
                "SELECT DISTINCT DATE(timestamp)
                 FROM activity_logs
                 ORDER BY 1 DESC
                 LIMIT $1"
            )
            .bind(limit.max(1))
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn get_active_vs_idle_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, i64, i64)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, i64, i64)>(
            "SELECT device_id,
                    COALESCE(SUM(active_seconds), 0)::BIGINT,
                    COALESCE(SUM(idle_seconds), 0)::BIGINT
             FROM input_activity_metrics
             WHERE timestamp >= $1
             GROUP BY device_id
             ORDER BY 2 DESC"
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_activity_logs_for_export(
        &self,
        device_id: Option<Uuid>,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let rows = if let Some(did) = device_id {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp
                 FROM activity_logs
                 WHERE device_id = $1 AND DATE(timestamp) BETWEEN $2 AND $3
                 ORDER BY timestamp DESC"
            )
            .bind(did)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp
                 FROM activity_logs
                 WHERE DATE(timestamp) BETWEEN $1 AND $2
                 ORDER BY timestamp DESC"
            )
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
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

    pub async fn get_live_devices_activity(&self) -> Result<Vec<LiveDeviceActivity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, i64, DateTime<Utc>)>(
            "SELECT DISTINCT ON (device_id)
                    device_id, app_name, window_title, duration_seconds, timestamp
             FROM activity_logs
             ORDER BY device_id, timestamp DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(device_id, app_name, window_title, duration_seconds, timestamp)| LiveDeviceActivity {
                device_id,
                app_name,
                window_title,
                duration_seconds,
                timestamp,
            })
            .collect())
    }

    // Inventory Methods
    pub async fn insert_inventory(
        &self,
        device_id: String,
        app_name: String,
        version: String,
        exe_hash: String,
    ) -> Result<InventoryItem, sqlx::Error> {
        let timestamp = Utc::now();

        let insert_result = sqlx::query(
            r#"
            INSERT INTO inventory (id, device_id, app_name, version, exe_hash, timestamp)
            SELECT $1, $2, $3, $4, $5, $6
            WHERE NOT EXISTS (
                SELECT 1
                FROM inventory
                WHERE device_id = $2
                  AND app_name = $3
                  AND COALESCE(version, '') = COALESCE($4, '')
                  AND COALESCE(exe_hash, '') = COALESCE($5, '')
            )
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&device_id)
        .bind(&app_name)
        .bind(&version)
        .bind(&exe_hash)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
            r#"
            SELECT id::text, device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
            FROM inventory
            WHERE device_id = $1
              AND app_name = $2
              AND COALESCE(version, '') = COALESCE($3, '')
              AND COALESCE(exe_hash, '') = COALESCE($4, '')
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(&device_id)
        .bind(&app_name)
        .bind(&version)
        .bind(&exe_hash)
        .fetch_one(&self.pool)
        .await?;

        if insert_result.rows_affected() > 0 {
            tracing::info!("📦 Saved inventory: {} | {} v{}", device_id, app_name, version);
        } else {
            tracing::debug!("📦 Skipped duplicate inventory item: {} | {} v{}", device_id, app_name, version);
        }

        let (id, device_id, app_name, version, exe_hash, timestamp) = row;

        Ok(InventoryItem {
            id,
            device_id,
            app_name,
            version,
            exe_hash,
            timestamp,
        })
    }

    pub async fn get_inventory(&self, device_id: Option<&str>) -> Result<Vec<InventoryItem>, sqlx::Error> {
        let items = if let Some(did) = device_id {
            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT DISTINCT ON (app_name, COALESCE(version, ''), COALESCE(exe_hash, ''))
                        id::text, device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
                 FROM inventory
                 WHERE device_id = $1
                 ORDER BY app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp DESC"
            )
            .bind(did)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT DISTINCT ON (device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''))
                        id::text, device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
                 FROM inventory
                 ORDER BY device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp DESC"
            )
            .fetch_all(&self.pool)
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

    pub async fn insert_usb_event(
        &self,
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
        .execute(&self.pool)
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
        &self,
        device_id: Option<Uuid>,
        limit: Option<i64>,
    ) -> Result<Vec<UsbEvent>, sqlx::Error> {
        let rows = if let Some(did) = device_id {
            if let Some(limit_value) = limit {
                sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, Option<String>, DateTime<Utc>)>(
                    "SELECT id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp FROM usb_events WHERE device_id = $1 ORDER BY timestamp DESC LIMIT $2"
                )
                .bind(did)
                .bind(limit_value.max(1))
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, Option<String>, DateTime<Utc>)>(
                    "SELECT id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp FROM usb_events WHERE device_id = $1 ORDER BY timestamp DESC"
                )
                .bind(did)
                .fetch_all(&self.pool)
                .await?
            }
        } else if let Some(limit_value) = limit {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, Option<String>, DateTime<Utc>)>(
                "SELECT id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp FROM usb_events ORDER BY timestamp DESC LIMIT $1"
            )
            .bind(limit_value.max(1))
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, Option<String>, DateTime<Utc>)>(
                "SELECT id, device_id, action, hardware_id, device_name, serial_number, volume_label, timestamp FROM usb_events ORDER BY timestamp DESC"
            )
            .fetch_all(&self.pool)
            .await?
        };

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

    pub async fn insert_input_summary(
        &self,
        device_id: String,
        active_seconds: i64,
        idle_seconds: i64,
        status: String,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        sqlx::query(
            r#"
            INSERT INTO input_activity_metrics (id, device_id, timestamp, active_seconds, idle_seconds, status)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(device_uuid)
        .bind(timestamp)
        .bind(active_seconds.max(0))
        .bind(idle_seconds.max(0))
        .bind(&status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_device_time_totals_today(&self) -> Result<Vec<DeviceTimeTotals>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid, i64, i64)>(
            "SELECT device_id, COALESCE(SUM(active_seconds), 0)::BIGINT, COALESCE(SUM(idle_seconds), 0)::BIGINT
             FROM input_activity_metrics
             WHERE timestamp >= date_trunc('day', NOW())
             GROUP BY device_id"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(device_id, active_seconds, idle_seconds)| DeviceTimeTotals {
                device_id,
                active_seconds,
                idle_seconds,
            })
            .collect())
    }

    pub async fn get_single_device_time_totals_today(
        &self,
        device_id: Uuid,
    ) -> Result<DeviceTimeTotals, sqlx::Error> {
        let row = sqlx::query_as::<_, (i64, i64)>(
            "SELECT COALESCE(SUM(active_seconds), 0)::BIGINT, COALESCE(SUM(idle_seconds), 0)::BIGINT
             FROM input_activity_metrics
             WHERE device_id = $1 AND timestamp >= date_trunc('day', NOW())"
        )
        .bind(device_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(DeviceTimeTotals {
            device_id,
            active_seconds: row.0,
            idle_seconds: row.1,
        })
    }

    // Device Methods
    pub async fn register_device(
        &self,
        hostname: String,
        device_id: String,
        mac_address: Option<String>,
        nickname: Option<String>,
    ) -> Result<Device, sqlx::Error> {
        let last_seen = Utc::now();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        sqlx::query(
            r#"
            INSERT INTO devices (device_id, hostname, nickname, mac_address, last_seen)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (device_id) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                nickname = COALESCE(EXCLUDED.nickname, devices.nickname),
                mac_address = COALESCE(EXCLUDED.mac_address, devices.mac_address),
                last_seen = EXCLUDED.last_seen,
                updated_at = NOW()
            "#,
        )
        .bind(device_uuid)
        .bind(&hostname)
        .bind(&nickname)
        .bind(&mac_address)
        .bind(last_seen)
        .execute(&self.pool)
        .await?;

        tracing::info!("🖥️  Registered device: {} ({})", hostname, device_id);

        Ok(Device {
            id: device_uuid,
            hostname,
            device_id: device_uuid,
            mac_address,
            created_at: last_seen,
            last_seen,
            nickname,
        })
    }

    pub async fn get_devices(&self) -> Result<Vec<Device>, sqlx::Error> {
        let devices = sqlx::query_as::<_, (Uuid, String, Uuid, Option<String>, DateTime<Utc>, DateTime<Utc>, Option<String>)>(
            "SELECT device_id AS id, hostname, device_id, nickname, last_seen, created_at, mac_address FROM devices ORDER BY last_seen DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(devices
            .into_iter()
            .map(|(id, hostname, device_id, nickname, last_seen, created_at, mac_address)| Device {
                id,
                hostname,
                device_id,
                mac_address,
                created_at,
                last_seen,
                nickname,
            })
            .collect())
    }

    pub async fn update_device_seen(&self, device_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE devices SET last_seen = NOW() WHERE device_id = $1")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        tracing::debug!("⏱️  Updated device last_seen: {}", device_id);
        Ok(())
    }

    /// Get overview statistics for the dashboard (devices today, active time, idle time, etc.)
    pub async fn get_overview(&self) -> Result<Overview, sqlx::Error> {
        // Count unique devices that have activity today
        let devices_today_result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT device_id) FROM activity_logs WHERE timestamp >= NOW() - INTERVAL '1 day'"
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(0);

        // Sum of duration_seconds for today (active time)
        let active_time_result = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(duration_seconds), 0)::BIGINT FROM activity_logs WHERE timestamp >= NOW() - INTERVAL '1 day' AND window_title NOT LIKE '%idle%'"
        )
        .fetch_one(&self.pool)
        .await?;

        // Sum of duration_seconds for idle activity today
        let idle_time_result = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(duration_seconds), 0)::BIGINT FROM activity_logs WHERE timestamp >= NOW() - INTERVAL '1 day' AND window_title LIKE '%idle%'"
        )
        .fetch_one(&self.pool)
        .await?;

        // Calculate idle percentage
        let total_time = active_time_result + idle_time_result;
        let idle_pct = if total_time > 0 {
            (idle_time_result as f64 / total_time as f64) * 100.0
        } else {
            0.0
        };

        // For now, keys_today is a placeholder (would need keystroke tracking in the agent)
        let keys_today = 0i64;

        Ok(Overview {
            devices_today: devices_today_result,
            active_time: active_time_result,
            idle_time: idle_time_result,
            idle_pct,
            keys_today,
        })
    }

    /// Get top 6 most used applications in the last N days
    pub async fn get_top_apps(&self, days: i64) -> Result<Vec<TopApp>, sqlx::Error> {
        let apps = sqlx::query_as::<_, (String, i64)>(
            "SELECT app_name, COALESCE(SUM(duration_seconds), 0)::BIGINT as total_duration FROM activity_logs 
             WHERE timestamp >= NOW() - INTERVAL '1 day' * $1 
             GROUP BY app_name 
             ORDER BY total_duration DESC 
             LIMIT 6"
        )
        .bind(days)
        .fetch_all(&self.pool)
        .await?;

        Ok(apps
            .into_iter()
            .map(|(app_name, total_duration_seconds)| TopApp {
                app_name,
                total_duration_seconds,
            })
            .collect())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
