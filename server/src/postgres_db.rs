// PostgreSQL database module for real data storage
use sqlx::{PgPool, postgres::PgPoolOptions};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: String,
    pub device_id: String,
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
pub struct Device {
    pub id: String,
    pub hostname: String,
    pub device_id: String,
    pub last_seen: DateTime<Utc>,
    pub nickname: Option<String>,
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
                id UUID PRIMARY KEY,
                device_id VARCHAR(255) UNIQUE NOT NULL,
                hostname VARCHAR(255) NOT NULL,
                nickname VARCHAR(255),
                last_seen TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
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

        Ok(())
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

        sqlx::query(
            r#"
            INSERT INTO activity_logs (id, device_id, app_name, window_title, duration_seconds, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(&device_id)
        .bind(&app_name)
        .bind(&window_title)
        .bind(duration_seconds)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        tracing::info!("📝 Saved activity log: {} | {} | {} seconds", device_id, app_name, duration_seconds);

        Ok(ActivityLog {
            id: id.to_string(),
            device_id,
            app_name,
            window_title,
            duration_seconds,
            timestamp,
        })
    }

    pub async fn get_activity_logs(&self, device_id: Option<&str>) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let logs = if let Some(did) = device_id {
            sqlx::query_as::<_, (String, String, String, String, i64, DateTime<Utc>)>(
                "SELECT id::text, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs WHERE device_id = $1 ORDER BY timestamp DESC"
            )
            .bind(did)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, String, String, String, i64, DateTime<Utc>)>(
                "SELECT id::text, device_id, app_name, window_title, duration_seconds, timestamp FROM activity_logs ORDER BY timestamp DESC"
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

    // Inventory Methods
    pub async fn insert_inventory(
        &self,
        device_id: String,
        app_name: String,
        version: String,
        exe_hash: String,
    ) -> Result<InventoryItem, sqlx::Error> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO inventory (id, device_id, app_name, version, exe_hash, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(&device_id)
        .bind(&app_name)
        .bind(&version)
        .bind(&exe_hash)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        tracing::info!("📦 Saved inventory: {} | {} v{}", device_id, app_name, version);

        Ok(InventoryItem {
            id: id.to_string(),
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
                "SELECT id::text, device_id, app_name, version, exe_hash, timestamp FROM inventory WHERE device_id = $1 ORDER BY timestamp DESC"
            )
            .bind(did)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT id::text, device_id, app_name, version, exe_hash, timestamp FROM inventory ORDER BY timestamp DESC"
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

    // Device Methods
    pub async fn register_device(
        &self,
        hostname: String,
        device_id: String,
        nickname: Option<String>,
    ) -> Result<Device, sqlx::Error> {
        let id = Uuid::new_v4();
        let last_seen = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO devices (id, device_id, hostname, nickname, last_seen)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (device_id) DO UPDATE SET last_seen = $5
            "#,
        )
        .bind(id)
        .bind(&device_id)
        .bind(&hostname)
        .bind(&nickname)
        .bind(last_seen)
        .execute(&self.pool)
        .await?;

        tracing::info!("🖥️  Registered device: {} ({})", hostname, device_id);

        Ok(Device {
            id: id.to_string(),
            hostname,
            device_id,
            last_seen,
            nickname,
        })
    }

    pub async fn get_devices(&self) -> Result<Vec<Device>, sqlx::Error> {
        let devices = sqlx::query_as::<_, (String, String, String, Option<String>, DateTime<Utc>)>(
            "SELECT id::text, hostname, device_id, nickname, last_seen FROM devices ORDER BY last_seen DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(devices
            .into_iter()
            .map(|(id, hostname, device_id, nickname, last_seen)| Device {
                id,
                hostname,
                device_id,
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
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}
