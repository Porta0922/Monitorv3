// PostgreSQL database module for real data storage
use sqlx::{PgPool, Postgres, QueryBuilder, postgres::PgPoolOptions};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

pub use crate::domains::device::models::*;
pub use crate::domains::activity::models::*;
pub use crate::domains::inventory::models::*;
pub use crate::domains::usb::models::*;
pub use crate::domains::wifi::models::*;
pub use crate::domains::security::models::*;













#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct StreamEvent {
    pub device_id: String,
    pub app: String,
    pub title: String,
    pub is_idle: bool,
    pub is_live: bool,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AgentCommand {
    pub id: Uuid,
    pub device_id: Uuid,
    pub command: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub acked_at: Option<DateTime<Utc>>,
    pub result: Option<serde_json::Value>,
}





pub struct Database {
    pub pool: PgPool,
}




impl Database {
    /// Create connection pool to PostgreSQL
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .after_connect(|connection,_|Box::pin(async move {sqlx::query("SET TIME ZONE 'UTC'").execute(connection).await?;Ok(())}))
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        if std::env::var("MONITOR_ENV").as_deref()==Ok("production") {
            // Application credentials cannot run migrations. Verify every embedded
            // version and checksum before accepting traffic.
            for migration in sqlx::migrate!("./migrations").iter() {
                let applied=sqlx::query_as::<_,(bool,Vec<u8>)>("SELECT success,checksum FROM _sqlx_migrations WHERE version=$1")
                    .bind(migration.version).fetch_optional(&pool).await?;
                if !applied.is_some_and(|(success,checksum)|success && checksum.as_slice()==migration.checksum.as_ref()) {
                    return Err(sqlx::Error::Protocol("Schema missing or checksum differs; run the migration job".into()));
                }
            }
        } else { Self::init_schema(&pool).await?; }

        Ok(Self { pool })
    }

    pub async fn migrate_production(database_url:&str)->Result<(),sqlx::Error> {
        let pool=PgPoolOptions::new().max_connections(1).connect(database_url).await?;
        Self::init_schema(&pool).await?;
        sqlx::raw_sql(include_str!("../../scripts/production/grants.sql")).execute(&pool).await?;
        pool.close().await;
        Ok(())
    }

    /// Apply embedded, checksummed migrations under SQLx's database lock.
    async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .map_err(|error| sqlx::Error::Migrate(Box::new(error)))
    }

    pub async fn register_processed_event(
        &self,
        event_id: &str,
        device_id: &str,
        event_type: &str,
        sequence: Option<i64>,
        boot_id: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.register_processed_event(event_id, device_id, event_type, sequence, boot_id).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
    }

    // Activity Methods
    pub async fn insert_activity_log(
        &self,
        device_id: String,
        app_name: String,
        window_title: String,
        duration_seconds: i64,
    ) -> Result<ActivityLog, sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_activity_log(device_id, app_name, window_title, duration_seconds).await?;
        writer.commit().await?;
        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn get_activity_logs(&self, device_id: Option<Uuid>) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let logs = if let Some(did) = device_id {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM desktop_activity WHERE device_id = $1 ORDER BY timestamp DESC"
            )
            .bind(did)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM desktop_activity ORDER BY timestamp DESC"
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
            "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp FROM desktop_activity"
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
        tz_offset_minutes: i32,
    ) -> Result<Vec<(String, String, i64, i64)>, sqlx::Error> {
        sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT app_name, window_title, COALESCE(SUM(duration_seconds), 0)::BIGINT, COUNT(*)::BIGINT
             FROM desktop_activity
             WHERE device_id = $1 AND DATE(timestamp + ($3 * INTERVAL '1 minute')) = $2
             GROUP BY app_name, window_title
             ORDER BY 3 DESC"
        )
        .bind(device_id)
        .bind(date)
        .bind(tz_offset_minutes)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_device_programs_by_hour_for_date(
        &self,
        device_id: Uuid,
        date: NaiveDate,
        tz_offset_minutes: i32,
    ) -> Result<Vec<(i32, String, String, i64, i64)>, sqlx::Error> {
        sqlx::query_as::<_, (i32, String, String, i64, i64)>(
            "SELECT EXTRACT(HOUR FROM (timestamp + ($3 * INTERVAL '1 minute')))::INT,
                    app_name,
                    COALESCE(window_title, ''),
                    COALESCE(SUM(duration_seconds), 0)::BIGINT,
                    COUNT(*)::BIGINT
             FROM desktop_activity
             WHERE device_id = $1
               AND DATE(timestamp + ($3 * INTERVAL '1 minute')) = $2
               GROUP BY 1, 2, 3
               ORDER BY 1 ASC, 4 DESC"
        )
        .bind(device_id)
        .bind(date)
        .bind(tz_offset_minutes)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_device_hourly_for_date(
        &self,
        device_id: Uuid,
        date: NaiveDate,
        tz_offset_minutes: i32,
    ) -> Result<Vec<(i32, i64, i64)>, sqlx::Error> {
        sqlx::query_as::<_, (i32, i64, i64)>(
            "SELECT EXTRACT(HOUR FROM (timestamp + ($3 * INTERVAL '1 minute')))::INT,
                    COALESCE(SUM(CASE WHEN LOWER(app_name) LIKE '%idle%' OR LOWER(window_title) LIKE '%idle%' THEN 0 ELSE duration_seconds END), 0)::BIGINT,
                    COALESCE(SUM(CASE WHEN LOWER(app_name) LIKE '%idle%' OR LOWER(window_title) LIKE '%idle%' THEN duration_seconds ELSE 0 END), 0)::BIGINT
             FROM desktop_activity
             WHERE device_id = $1 AND DATE(timestamp + ($3 * INTERVAL '1 minute')) = $2
             GROUP BY 1
             ORDER BY 1"
        )
        .bind(device_id)
        .bind(date)
        .bind(tz_offset_minutes)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_available_dates(
        &self,
        device_id: Option<Uuid>,
        limit: i64,
        tz_offset_minutes: i32,
    ) -> Result<Vec<NaiveDate>, sqlx::Error> {
        if let Some(did) = device_id {
            sqlx::query_scalar::<_, NaiveDate>(
                "SELECT DISTINCT DATE(timestamp + ($2 * INTERVAL '1 minute'))
                 FROM desktop_activity
                 WHERE device_id = $1
                 ORDER BY 1 DESC
                 LIMIT $3"
            )
            .bind(did)
            .bind(tz_offset_minutes)
            .bind(limit.max(1))
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_scalar::<_, NaiveDate>(
                "SELECT DISTINCT DATE(timestamp + ($1 * INTERVAL '1 minute'))
                 FROM desktop_activity
                 ORDER BY 1 DESC
                 LIMIT $2"
            )
            .bind(tz_offset_minutes)
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
                    COALESCE(SUM(active_person_seconds), 0)::BIGINT,
                    COALESCE(SUM(idle_person_seconds), 0)::BIGINT
             FROM session_activity_combined
             WHERE day >= ($1::timestamptz AT TIME ZONE 'UTC')::date AND available_samples>0
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
        tz_offset_minutes: i32,
    ) -> Result<Vec<ActivityLog>, sqlx::Error> {
        let rows = if let Some(did) = device_id {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp
                 FROM desktop_activity
                 WHERE device_id = $1 AND DATE(timestamp + ($4 * INTERVAL '1 minute')) BETWEEN $2 AND $3
                 ORDER BY timestamp DESC"
            )
            .bind(did)
            .bind(from)
            .bind(to)
            .bind(tz_offset_minutes)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, Uuid, String, String, i64, DateTime<Utc>)>(
                "SELECT id, device_id, app_name, window_title, duration_seconds, timestamp
                 FROM desktop_activity
                 WHERE DATE(timestamp + ($3 * INTERVAL '1 minute')) BETWEEN $1 AND $2
                 ORDER BY timestamp DESC"
            )
            .bind(from)
            .bind(to)
            .bind(tz_offset_minutes)
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

    pub async fn update_device_nickname(
        &self,
        device_id: Uuid,
        nickname: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE devices SET nickname = $2, updated_at = NOW() WHERE device_id = $1"
        )
        .bind(device_id)
        .bind(nickname)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_audit_event(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        details: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO audit_events (actor, action, target, details) VALUES ($1, $2, $3, $4)"
        )
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(details)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_audit_events(&self, limit: i64) -> Result<Vec<AuditEvent>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, Option<String>, DateTime<Utc>)>(
            "SELECT id, actor, action, target, details, created_at
             FROM audit_events
             ORDER BY created_at DESC
             LIMIT $1"
        )
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, actor, action, target, details, created_at)| AuditEvent {
                id,
                actor,
                action,
                target,
                details,
                created_at,
            })
            .collect())
    }

    pub async fn insert_security_alert(
        &self,
        device_id: Uuid,
        alert_type: &str,
        app_name: Option<&str>,
        exe_hash: Option<&str>,
        description: &str,
        severity: &str,
    ) -> Result<SecurityAlert, sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_security_alert(device_id, alert_type, app_name, exe_hash, description, severity).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_security_alerts(
        &self,
        device_id: Option<Uuid>,
        severity: Option<&str>,
        resolved: Option<bool>,
        limit: i64,
    ) -> Result<Vec<SecurityAlert>, sqlx::Error> {
        let mut query_builder = QueryBuilder::<Postgres>::new(
            "SELECT id::BIGINT, device_id, alert_type, app_name, exe_hash, description, severity, resolved, created_at FROM security_alerts"
        );

        let mut has_where = false;

        if let Some(device_uuid) = device_id {
            query_builder.push(" WHERE device_id = ").push_bind(device_uuid);
            has_where = true;
        }

        if let Some(severity_value) = severity {
            if has_where {
                query_builder.push(" AND severity = ");
            } else {
                query_builder.push(" WHERE severity = ");
                has_where = true;
            }
            query_builder.push_bind(severity_value);
        }

        if let Some(resolved_value) = resolved {
            if has_where {
                query_builder.push(" AND resolved = ");
            } else {
                query_builder.push(" WHERE resolved = ");
            }
            query_builder.push_bind(resolved_value);
        }

        query_builder.push(" ORDER BY created_at DESC LIMIT ").push_bind(limit.max(1));

        let rows = query_builder
            .build_query_as::<(i64, Uuid, String, String, String, String, String, bool, DateTime<Utc>)>()
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|(id, device_id, alert_type, app_name, exe_hash, description, severity, resolved, created_at)| SecurityAlert {
                id,
                device_id,
                alert_type,
                app_name,
                exe_hash,
                description,
                severity,
                resolved,
                created_at,
            })
            .collect())
    }

    pub async fn resolve_security_alert(
        &self,
        alert_id: i64,
        resolution_notes: Option<&str>,
    ) -> Result<Option<SecurityAlert>, sqlx::Error> {
        let row = sqlx::query_as::<_, (i64, Uuid, String, String, String, String, String, bool, DateTime<Utc>)>(
            r#"
            UPDATE security_alerts
            SET resolved = TRUE,
                resolution_notes = COALESCE($2, resolution_notes),
                resolved_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND resolved IS FALSE
            RETURNING id::BIGINT, device_id, alert_type, app_name, exe_hash, description, severity, resolved, created_at
            "#,
        )
        .bind(alert_id)
        .bind(resolution_notes)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, device_id, alert_type, app_name, exe_hash, description, severity, resolved, created_at)| SecurityAlert {
            id,
            device_id,
            alert_type,
            app_name,
            exe_hash,
            description,
            severity,
            resolved,
            created_at,
        }))
    }

    pub async fn insert_process_termination_attempt(
        &self,
        device_id: Uuid,
        method: &str,
        attempted_by: Option<&str>,
        blocked: bool,
        auto_restarted: bool,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_process_termination_attempt(device_id, method, attempted_by, blocked, auto_restarted, timestamp).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_operational_metrics(
        &self,
        online_threshold_seconds: i64,
        stale_threshold_seconds: i64,
    ) -> Result<OperationalMetrics, sqlx::Error> {
        let devices_total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::BIGINT FROM devices")
            .fetch_one(&self.pool)
            .await?
            .max(0);

        let devices_online = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM devices WHERE last_seen >= NOW() - ($1 * INTERVAL '1 second')"
        )
        .bind(online_threshold_seconds.max(1))
        .fetch_one(&self.pool)
        .await?
        .max(0);

        let devices_stale = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM devices WHERE last_seen < NOW() - ($1 * INTERVAL '1 second')"
        )
        .bind(stale_threshold_seconds.max(1))
        .fetch_one(&self.pool)
        .await?
        .max(0);

        let activities_last_hour = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM desktop_activity WHERE timestamp >= NOW() - INTERVAL '1 hour'"
        )
        .fetch_one(&self.pool)
        .await?
        .max(0);

        let input_rows_today = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(sum(available_samples+unavailable_samples+unknown_samples),0)::bigint FROM session_activity_combined WHERE day=(now() AT TIME ZONE 'UTC')::date"
        )
        .fetch_one(&self.pool)
        .await?
        .max(0);

        let newest_activity_age_seconds = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(EXTRACT(EPOCH FROM (NOW() - MAX(timestamp)))::BIGINT, 0) FROM desktop_activity"
        )
        .fetch_one(&self.pool)
        .await?
        .max(0);

        Ok(OperationalMetrics {
            devices_total,
            devices_online,
            devices_stale,
            activities_last_hour,
            input_rows_today,
            newest_activity_age_seconds,
        })
    }

    pub async fn get_live_devices_activity(&self) -> Result<Vec<LiveDeviceActivity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, i64, DateTime<Utc>)>(
            "SELECT
                    d.device_id,
                    COALESCE(a.app_name, 'Sin actividad') AS app_name,
                    COALESCE(a.window_title, 'Heartbeat activo') AS window_title,
                    COALESCE(a.duration_seconds, 0) AS duration_seconds,
                    COALESCE(a.timestamp, d.last_seen) AS timestamp
             FROM devices d
             LEFT JOIN LATERAL (
                 SELECT app_name, window_title, duration_seconds, timestamp
                 FROM desktop_activity
                 WHERE device_id = d.device_id
                 ORDER BY timestamp DESC
                 LIMIT 1
             ) a ON TRUE
             ORDER BY d.last_seen DESC"
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

    pub async fn insert_node_resource_metric(
        &self,
        device_id: String,
        timestamp: DateTime<Utc>,
        cpu_percent: f64,
        memory_used_mb: f64,
        memory_percent: f64,
        top_process_name: Option<String>,
        top_process_cpu_percent: Option<f64>,
        top_process_memory_mb: Option<f64>,
    ) -> Result<(), sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_node_resource_metric(device_id, timestamp, cpu_percent, memory_used_mb, memory_percent, top_process_name, top_process_cpu_percent, top_process_memory_mb).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_device_resource_metrics_for_date(
        &self,
        device_id: Uuid,
        date: NaiveDate,
        tz_offset_minutes: i32,
        limit: i64,
    ) -> Result<Vec<NodeResourceMetric>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (DateTime<Utc>, f64, f64, f64, Option<String>, Option<f64>, Option<f64>)>(
            "SELECT timestamp,
                    COALESCE(cpu_percent, 0),
                    COALESCE(memory_used_mb, 0),
                    COALESCE(memory_percent, 0),
                    top_process_name,
                    CASE
                        WHEN top_process_cpu_percent IS NULL THEN NULL
                        ELSE LEAST(100.0, GREATEST(0.0, top_process_cpu_percent))
                    END,
                    CASE
                        WHEN top_process_memory_mb IS NULL THEN NULL
                        WHEN top_process_memory_mb > 8192 THEN top_process_memory_mb / 1024.0
                        ELSE GREATEST(0.0, top_process_memory_mb)
                    END
             FROM node_resource_metrics
             WHERE device_id = $1
               AND DATE(timestamp + ($3 * INTERVAL '1 minute')) = $2
             ORDER BY timestamp ASC
             LIMIT $4"
        )
        .bind(device_id)
        .bind(date)
        .bind(tz_offset_minutes)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    timestamp,
                    cpu_percent,
                    memory_used_mb,
                    memory_percent,
                    top_process_name,
                    top_process_cpu_percent,
                    top_process_memory_mb,
                )| NodeResourceMetric {
                    timestamp,
                    cpu_percent,
                    memory_used_mb,
                    memory_percent,
                    top_process_name,
                    top_process_cpu_percent,
                    top_process_memory_mb,
                },
            )
            .collect())
    }

    pub async fn get_resource_peaks_today(
        &self,
        limit: i64,
    ) -> Result<Vec<DeviceResourcePeak>, sqlx::Error> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                f64,
                f64,
                f64,
                f64,
                Option<String>,
                Option<f64>,
                Option<f64>,
                DateTime<Utc>,
            ),
        >(
            r#"
            WITH today AS (
                SELECT *
                FROM node_resource_metrics
                WHERE timestamp >= date_trunc('day', NOW())
            ),
            peaks AS (
                SELECT
                    device_id,
                    MAX(COALESCE(cpu_percent, 0)) AS peak_cpu_percent,
                    MAX(COALESCE(memory_percent, 0)) AS peak_memory_percent
                FROM today
                GROUP BY device_id
            ),
            latest AS (
                SELECT DISTINCT ON (device_id)
                    device_id,
                    COALESCE(cpu_percent, 0) AS last_cpu_percent,
                    COALESCE(memory_percent, 0) AS last_memory_percent,
                    top_process_name,
                    CASE
                        WHEN top_process_cpu_percent IS NULL THEN NULL
                        ELSE LEAST(100.0, GREATEST(0.0, top_process_cpu_percent))
                    END AS top_process_cpu_percent,
                    CASE
                        WHEN top_process_memory_mb IS NULL THEN NULL
                        WHEN top_process_memory_mb > 8192 THEN top_process_memory_mb / 1024.0
                        ELSE GREATEST(0.0, top_process_memory_mb)
                    END AS top_process_memory_mb,
                    timestamp
                FROM today
                ORDER BY device_id, timestamp DESC
            )
            SELECT
                p.device_id,
                p.peak_cpu_percent,
                p.peak_memory_percent,
                l.last_cpu_percent,
                l.last_memory_percent,
                l.top_process_name,
                l.top_process_cpu_percent,
                l.top_process_memory_mb,
                l.timestamp
            FROM peaks p
            JOIN latest l ON l.device_id = p.device_id
            ORDER BY p.peak_cpu_percent DESC, p.peak_memory_percent DESC
            LIMIT $1
            "#,
        )
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    device_id,
                    peak_cpu_percent,
                    peak_memory_percent,
                    last_cpu_percent,
                    last_memory_percent,
                    top_process_name,
                    top_process_cpu_percent,
                    top_process_memory_mb,
                    last_seen,
                )| DeviceResourcePeak {
                    device_id,
                    peak_cpu_percent,
                    peak_memory_percent,
                    last_cpu_percent,
                    last_memory_percent,
                    top_process_name,
                    top_process_cpu_percent,
                    top_process_memory_mb,
                    last_seen,
                },
            )
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
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_inventory(device_id, app_name, version, exe_hash).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_inventory(&self, device_id: Option<&str>) -> Result<Vec<InventoryItem>, sqlx::Error> {
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
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
                "SELECT DISTINCT ON (device_id, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''))
                        id::text, device_id::text, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
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

    pub async fn replace_running_apps_snapshot(
        &self,
        device_id: String,
        apps: Vec<(String, String, i32, Option<String>, Option<String>)>,
    ) -> Result<(), sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.replace_running_apps_snapshot(device_id, apps).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_running_apps(&self, device_id: Uuid) -> Result<Vec<RunningAppItem>, sqlx::Error> {
        crate::domains::inventory::repository::get_running_apps(&self.pool, device_id).await
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
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_usb_event(device_id, action, hardware_id, device_name, serial_number, volume_label, Utc::now()).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_usb_events(
        &self,
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
        .fetch_all(&self.pool)
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

    pub async fn insert_wifi_event(
        &self,
        device_id: String,
        interface_name: String,
        state: String,
        ssid: Option<String>,
        bssid: Option<String>,
        signal_percent: Option<i32>,
        timestamp: DateTime<Utc>,
    ) -> Result<WifiEvent, sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_wifi_event(device_id, interface_name, state, ssid, bssid, signal_percent, timestamp).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_wifi_events(
        &self,
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
        .fetch_all(&self.pool)
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

    pub async fn insert_input_summary(
        &self,
        device_id: String,
        timestamp: DateTime<Utc>,
        active_seconds: i64,
        idle_seconds: i64,
        keys_count: i64,
        mouse_moves_count: i64,
        clicks_count: i64,
        status: String,
        bucket_max_seconds: i64,
    ) -> Result<(), sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_input_summary(device_id, timestamp, active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count, status, bucket_max_seconds, None).await?;
        writer.commit().await?;
        Ok(result)
    }

    // Device Methods
    pub async fn register_device(
        &self,
        hostname: String,
        device_id: String,
        mac_address: Option<String>,
        nickname: Option<String>,
        version: Option<String>,
    ) -> Result<Device, sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.register_device(hostname, device_id, mac_address, nickname, version).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_devices(&self) -> Result<Vec<Device>, sqlx::Error> {
        let devices = sqlx::query_as::<_, (Uuid, String, Uuid, Option<String>, DateTime<Utc>, DateTime<Utc>, Option<String>, Option<String>)>(
            "SELECT device_id AS id, hostname, device_id, nickname, last_seen, created_at, mac_address, version FROM devices ORDER BY last_seen DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(devices
            .into_iter()
            .map(|(id, hostname, device_id, nickname, last_seen, created_at, mac_address, version)| Device {
                id,
                hostname,
                device_id,
                mac_address,
                version,
                created_at,
                last_seen,
                nickname,
            })
            .collect())
    }

    pub async fn get_device_by_id(&self, device_id: Uuid) -> Result<Option<Device>, sqlx::Error> {
        let result = sqlx::query_as::<_, (Uuid, String, Uuid, Option<String>, DateTime<Utc>, DateTime<Utc>, Option<String>, Option<String>)>(
            "SELECT device_id AS id, hostname, device_id, nickname, last_seen, created_at, mac_address, version FROM devices WHERE device_id = $1"
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(id, hostname, device_id, nickname, last_seen, created_at, mac_address, version)| Device {
            id,
            hostname,
            device_id,
            mac_address,
            version,
            created_at,
            last_seen,
            nickname,
        }))
    }

    pub async fn insert_agent_command(
        &self,
        device_id: Uuid,
        command: &str,
        payload: serde_json::Value,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO agent_commands (id, device_id, command, payload, status)
            VALUES ($1, $2, $3, $4, 'pending')
            "#,
        )
        .bind(id)
        .bind(device_id)
        .bind(command)
        .bind(payload)
        .execute(&self.pool)
        .await?;

        tracing::info!("📟 Agent command queued: {} -> {} (id={})", device_id, command, id);
        Ok(id)
    }

    pub async fn get_pending_agent_commands(
        &self,
        device_id: Uuid,
    ) -> Result<Vec<AgentCommand>, sqlx::Error> {
        sqlx::query("UPDATE agent_commands SET status='expired' WHERE device_id=$1 AND status IN ('pending','delivered') AND created_at<=now()-interval '1 hour'").bind(device_id).execute(&self.pool).await?;
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, serde_json::Value, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<serde_json::Value>)>(
            "WITH selected AS (SELECT id FROM agent_commands WHERE device_id=$1 AND command IN ('ping','status','run_diagnostic') AND created_at>now()-interval '1 hour' AND (status='pending' OR (status='delivered' AND delivered_at<now()-interval '60 seconds')) ORDER BY created_at,id LIMIT 25 FOR UPDATE SKIP LOCKED)
             UPDATE agent_commands c SET status='delivered',delivered_at=now() FROM selected s WHERE c.id=s.id RETURNING c.id,c.device_id,c.command,c.payload,c.status,c.created_at,c.acked_at,c.result"
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id, device_id, command, payload, status, created_at, acked_at, result)| AgentCommand {
            id,
            device_id,
            command,
            payload,
            status,
            created_at,
            acked_at,
            result,
        }).collect())
    }

    pub async fn mark_agent_command_delivered(&self, command_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE agent_commands SET status = 'delivered', delivered_at = NOW() \
             WHERE id = $1 AND status = 'pending'"
        )
        .bind(command_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn ack_agent_command(&self,command_id:Uuid,ack_status:&str,result:serde_json::Value)->Result<bool,sqlx::Error> {
        let status=if ack_status=="ok"{"done"}else{"failed"};
        let changed=sqlx::query("UPDATE agent_commands SET status=$2,acked_at=now(),result=$3 WHERE id=$1 AND status='delivered' AND created_at>now()-interval '1 hour'").bind(command_id).bind(status).bind(&result).execute(&self.pool).await?;
        if changed.rows_affected()==1{return Ok(true);}
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_commands WHERE id=$1 AND status=$2 AND result=$3)").bind(command_id).bind(status).bind(result).fetch_one(&self.pool).await
    }

    pub async fn list_agent_commands(
        &self,
        device_id: Option<Uuid>,
    ) -> Result<Vec<AgentCommand>, sqlx::Error> {
        let rows = match device_id {
            Some(did) => {
                sqlx::query_as::<_, (Uuid, Uuid, String, serde_json::Value, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<serde_json::Value>)>(
                    "SELECT id, device_id, command, payload, status, created_at, acked_at, result \
                     FROM agent_commands WHERE device_id = $1 ORDER BY created_at DESC LIMIT 100"
                )
                .bind(did)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, (Uuid, Uuid, String, serde_json::Value, String, DateTime<Utc>, Option<DateTime<Utc>>, Option<serde_json::Value>)>(
                    "SELECT id, device_id, command, payload, status, created_at, acked_at, result \
                     FROM agent_commands ORDER BY created_at DESC LIMIT 100"
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(rows.into_iter().map(|(id, device_id, command, payload, status, created_at, acked_at, result)| AgentCommand {
            id,
            device_id,
            command,
            payload,
            status,
            created_at,
            acked_at,
            result,
        }).collect())
    }

    #[allow(dead_code)]
    pub async fn update_device_seen(&self, device_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE devices SET last_seen = NOW() WHERE device_id = $1")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        tracing::debug!("⏱️  Updated device last_seen: {}", device_id);
        Ok(())
    }

    /// Get top 6 most used applications in the last N days
    // -------------------------------------------------------------------------
    // Security Events
    // -------------------------------------------------------------------------

    pub async fn insert_security_event(
        &self,
        device_id: String,
        query_name: String,
        query_pack: Option<String>,
        mitre_technique: Option<String>,
        severity: String,
        raw_data: serde_json::Value,
        event_fingerprint: Option<String>,
        timestamp: DateTime<Utc>,
    ) -> Result<SecurityEvent, sqlx::Error> {
        let mut writer = crate::telemetry_writer::TelemetryWriter::begin(&self.pool).await?;
        let result = writer.insert_security_event(device_id, query_name, query_pack, mitre_technique, severity, raw_data, event_fingerprint, timestamp).await?;
        writer.commit().await?;
        Ok(result)
    }

    pub async fn get_security_events(
        &self,
        device_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        severity: Option<&str>,
        mitre_technique: Option<&str>,
        limit: i64,
    ) -> Result<Vec<SecurityEvent>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "SELECT id, timestamp, device_id, query_name, query_pack, mitre_technique, severity, raw_data, event_fingerprint, created_at FROM security_events WHERE 1=1"
        );
        if let Some(did) = device_id {
            qb.push(" AND device_id = ").push_bind(did);
        }
        if let Some(f) = from {
            qb.push(" AND timestamp >= ").push_bind(f);
        }
        if let Some(t) = to {
            qb.push(" AND timestamp <= ").push_bind(t);
        }
        if let Some(sev) = severity {
            qb.push(" AND severity = ").push_bind(sev.to_uppercase());
        }
        if let Some(tech) = mitre_technique {
            qb.push(" AND mitre_technique ILIKE ").push_bind(format!("%{}%", tech));
        }
        qb.push(" ORDER BY timestamp DESC LIMIT ").push_bind(limit.max(1));

        qb.build_query_as::<(i64, DateTime<Utc>, Uuid, String, Option<String>, Option<String>, String, serde_json::Value, Option<String>, DateTime<Utc>)>()
            .fetch_all(&self.pool)
            .await
            .map(|rows| rows.into_iter().map(|(id, ts, did, qn, qp, mt, sev, rd, fp, ca)| SecurityEvent {
                id, timestamp: ts, device_id: did,
                query_name: qn, query_pack: qp, mitre_technique: mt,
                severity: sev, raw_data: rd, event_fingerprint: fp, created_at: ca,
            }).collect())
    }

    pub async fn get_security_summary(
        &self,
        device_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<SecuritySummaryRow>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "SELECT severity, COALESCE(mitre_technique, 'UNKNOWN') AS mitre_technique, COUNT(*)::BIGINT FROM security_events WHERE 1=1"
        );
        if let Some(did) = device_id {
            qb.push(" AND device_id = ").push_bind(did);
        }
        if let Some(f) = from {
            qb.push(" AND timestamp >= ").push_bind(f);
        }
        if let Some(t) = to {
            qb.push(" AND timestamp <= ").push_bind(t);
        }
        qb.push(" GROUP BY severity, mitre_technique ORDER BY 3 DESC");

        qb.build_query_as::<(String, String, i64)>()
            .fetch_all(&self.pool)
            .await
            .map(|rows| rows.into_iter().map(|(severity, mitre_technique, event_count)| SecuritySummaryRow {
                severity, mitre_technique, event_count,
            }).collect())
    }

    pub async fn get_top_apps(&self, days: i64) -> Result<Vec<TopApp>, sqlx::Error> {
        let apps = sqlx::query_as::<_, (String, i64)>(
            "SELECT app_name, COALESCE(SUM(duration_seconds), 0)::BIGINT as total_duration FROM desktop_activity
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
#[cfg(test)]
mod migration_tests {
    use super::*;
    use sqlx::Executor;

    #[tokio::test]
    #[ignore = "requires a disposable TimescaleDB and LEGACY_SCHEMA_DIR"]
    async fn fresh_and_legacy_migrations_preserve_data() {
        let url = std::env::var("TEST_DATABASE_URL").expect("disposable database URL");
        let admin = PgPool::connect(&url).await.unwrap();
        for legacy in [false, true] {
            let name = format!("migration_test_{}", Uuid::new_v4().simple());
            admin.execute(format!("CREATE DATABASE {name}").as_str()).await.unwrap();
            let db_url = format!("{}/{}", url.rsplit_once('/').unwrap().0, name);
            let pool = PgPool::connect(&db_url).await.unwrap();
            let id = Uuid::new_v4();
            if legacy {
                let root = std::env::var("LEGACY_SCHEMA_DIR").unwrap();
                for file in ["001_init_schema.sql", "002_input_heatmaps_and_alerts.sql", "003_security_events.sql"] {
                    let sql = std::fs::read_to_string(format!("{root}/{file}")).unwrap();
                    pool.execute(sql.as_str()).await.unwrap();
                }
                sqlx::query("INSERT INTO devices(device_id,hostname,last_seen) VALUES ($1,'preserved',now())")
                    .bind(id).execute(&pool).await.unwrap();
                pool.execute("SELECT alter_job(job_id, scheduled => false) FROM timescaledb_information.jobs WHERE proc_name LIKE 'policy_%'").await.unwrap();
                sqlx::query("INSERT INTO activity_logs(device_id,app_name,duration_seconds,timestamp) SELECT $1,'legacy',42,now()-interval '40 days'+n*interval '1 second' FROM generate_series(1,100000) n")
                    .bind(id).execute(&pool).await.unwrap();
                pool.execute("SELECT compress_chunk(c) FROM show_chunks('activity_logs') c").await.unwrap();
                let compressed: i64 = sqlx::query_scalar("SELECT count(*) FROM timescaledb_information.chunks WHERE hypertable_name='activity_logs' AND is_compressed")
                    .fetch_one(&pool).await.unwrap();
                assert!(compressed > 0);
                // Lock contention must abort migration rather than hang startup.
                let mut blocker = pool.begin().await.unwrap();
                sqlx::query("LOCK TABLE devices IN ACCESS EXCLUSIVE MODE").execute(&mut *blocker).await.unwrap();
                let blocked = tokio::time::timeout(std::time::Duration::from_secs(15), Database::connect(&db_url)).await.unwrap();
                assert!(blocked.is_err());
                blocker.rollback().await.unwrap();
                let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success").fetch_one(&pool).await.unwrap();
                assert_eq!(applied, 0);

            }
            let started = std::time::Instant::now();
            // Concurrent startup must serialize migration application.
            let (first, second) = tokio::join!(Database::connect(&db_url), Database::connect(&db_url));
            let first = first.unwrap();
            let second = second.unwrap();
            println!("Migration legacy={legacy}, elapsed={:?}", started.elapsed());
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE success")
                .fetch_one(&pool).await.unwrap();
            // The legacy fixture has three SQL files; the current application
            // must apply its entire embedded migration history exactly once.
            assert_eq!(count as usize, sqlx::migrate!("./migrations").iter().count());
            if legacy {
                let duration: i64 = sqlx::query_scalar("SELECT SUM(duration_seconds)::bigint FROM activity_logs WHERE device_id=$1")
                    .bind(id).fetch_one(&pool).await.unwrap();
                assert_eq!(duration, 4_200_000);
                let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM activity_logs WHERE id IS NOT NULL AND created_at IS NOT NULL")
                    .fetch_one(&pool).await.unwrap();
                assert_eq!(rows, 100_000);
                assert_eq!(first.get_devices().await.unwrap().len(), 1);
            }
            // These columns were missing or incompatible in the Docker schema.
            pool.execute("SELECT id::bigint, timestamp, resolved_at FROM security_alerts; SELECT auto_restarted FROM process_termination_attempts; SELECT id, created_at FROM activity_logs;")
                .await.unwrap();
            first.pool.close().await;
            second.pool.close().await;
            pool.close().await;
            admin.execute(format!("DROP DATABASE {name}").as_str()).await.unwrap();
        }
        admin.close().await;
    }
}
