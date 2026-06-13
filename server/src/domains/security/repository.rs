use super::models::*;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;
use chrono::*;


pub async fn insert_security_event(
        pool: &PgPool,
        device_id: String,
        query_name: String,
        query_pack: Option<String>,
        mitre_technique: Option<String>,
        severity: String,
        raw_data: serde_json::Value,
        event_fingerprint: Option<String>,
        timestamp: DateTime<Utc>,
    ) -> Result<SecurityEvent, sqlx::Error> {
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let row = sqlx::query_as::<_, (i64, DateTime<Utc>, Uuid, String, Option<String>, Option<String>, String, serde_json::Value, Option<String>, DateTime<Utc>)>(
            r#"
            INSERT INTO security_events
                (timestamp, device_id, query_name, query_pack, mitre_technique, severity, raw_data, event_fingerprint)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (event_fingerprint) WHERE event_fingerprint IS NOT NULL DO NOTHING
            RETURNING id, timestamp, device_id, query_name, query_pack, mitre_technique, severity, raw_data, event_fingerprint, created_at
            "#,
        )
        .bind(timestamp)
        .bind(device_uuid)
        .bind(&query_name)
        .bind(&query_pack)
        .bind(&mitre_technique)
        .bind(&severity)
        .bind(&raw_data)
        .bind(&event_fingerprint)
        .fetch_optional(pool)
        .await?;

        if let Some((id, ts, did, qn, qp, mt, sev, rd, fp, ca)) = row {
            return Ok(SecurityEvent {
                id, timestamp: ts, device_id: did,
                query_name: qn, query_pack: qp, mitre_technique: mt,
                severity: sev, raw_data: rd, event_fingerprint: fp, created_at: ca,
            });
        }

        // Event was deduplicated (ON CONFLICT DO NOTHING)
        sqlx::query_as::<_, (i64, DateTime<Utc>, Uuid, String, Option<String>, Option<String>, String, serde_json::Value, Option<String>, DateTime<Utc>)>(
            "SELECT id, timestamp, device_id, query_name, query_pack, mitre_technique, severity, raw_data, event_fingerprint, created_at FROM security_events WHERE event_fingerprint = $1"
        )
        .bind(&event_fingerprint)
        .fetch_one(pool)
        .await
        .map(|(id, ts, did, qn, qp, mt, sev, rd, fp, ca)| SecurityEvent {
            id, timestamp: ts, device_id: did,
            query_name: qn, query_pack: qp, mitre_technique: mt,
            severity: sev, raw_data: rd, event_fingerprint: fp, created_at: ca,
        })
    }

pub async fn get_security_events(
        pool: &PgPool,
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
            .fetch_all(pool)
            .await
            .map(|rows| rows.into_iter().map(|(id, ts, did, qn, qp, mt, sev, rd, fp, ca)| SecurityEvent {
                id, timestamp: ts, device_id: did,
                query_name: qn, query_pack: qp, mitre_technique: mt,
                severity: sev, raw_data: rd, event_fingerprint: fp, created_at: ca,
            }).collect())
    }

pub async fn insert_security_alert(
        pool: &PgPool,
        device_id: Uuid,
        alert_type: &str,
        app_name: Option<&str>,
        exe_hash: Option<&str>,
        description: &str,
        severity: &str,
    ) -> Result<SecurityAlert, sqlx::Error> {
        let row = sqlx::query_as::<_, (i64, Uuid, String, String, String, String, String, bool, DateTime<Utc>)>(
            r#"
            INSERT INTO security_alerts (device_id, alert_type, app_name, exe_hash, description, severity, resolved)
            VALUES ($1, $2, $3, $4, $5, $6, FALSE)
            RETURNING id::BIGINT, device_id, alert_type, app_name, exe_hash, description, severity, resolved, created_at
            "#,
        )
        .bind(device_id)
        .bind(alert_type)
        .bind(app_name.unwrap_or(""))
        .bind(exe_hash.unwrap_or(""))
        .bind(description)
        .bind(severity)
        .fetch_one(pool)
        .await?;

        Ok(SecurityAlert {
            id: row.0,
            device_id: row.1,
            alert_type: row.2,
            app_name: row.3,
            exe_hash: row.4,
            description: row.5,
            severity: row.6,
            resolved: row.7,
            created_at: row.8,
        })
    }

pub async fn get_security_alerts(
        pool: &PgPool,
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
            .fetch_all(pool)
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
        pool: &PgPool,
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
            WHERE id = $1
            RETURNING id::BIGINT, device_id, alert_type, app_name, exe_hash, description, severity, resolved, created_at
            "#,
        )
        .bind(alert_id)
        .bind(resolution_notes)
        .fetch_optional(pool)
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

pub async fn get_security_summary(
        pool: &PgPool,
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
            .fetch_all(pool)
            .await
            .map(|rows| rows.into_iter().map(|(severity, mitre_technique, event_count)| SecuritySummaryRow {
                severity, mitre_technique, event_count,
            }).collect())
    }

