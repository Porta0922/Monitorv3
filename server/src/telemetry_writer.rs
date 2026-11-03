//! One transaction for all effects of a telemetry event, including its deduplication marker.
use crate::postgres_db::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;
pub struct TelemetryWriter { tx: sqlx::Transaction<'static, sqlx::Postgres> }
impl TelemetryWriter {
    pub async fn insert_session_event(&mut self,event:&serde_json::Value)->Result<(),Box<dyn std::error::Error+Send+Sync>> {
        let device=event["device_id"].as_str().ok_or("Missing session device")?;
        let principal=event["source_principal"].as_str().filter(|s|!s.is_empty() && s.len()<=128 && s.bytes().all(|c|c.is_ascii_alphanumeric() || b"-_:".contains(&c))).ok_or("Invalid session principal")?;
        let session=Uuid::parse_str(event["source_session"].as_str().ok_or("Missing session scope")?)?;
        let id=Uuid::parse_str(event["event_id"].as_str().ok_or("Missing session event ID")?)?;
        let timestamp=DateTime::parse_from_rfc3339(event["timestamp"].as_str().ok_or("Missing session timestamp")?)?.with_timezone(&Utc);
        let kind=event["event_type"].as_str().ok_or("Missing session kind")?;
        if !matches!(kind,"input_summary"|"activity"|"running_apps"|"input_heatmap") || event["source_role"]!="session" || !event["payload"].is_object() || event["payload"].to_string().len()>262144 {return Err("Invalid session event".into());}
        if kind=="input_summary" {
            for key in ["keys_count","mouse_moves_count","clicks_count","active_seconds","idle_seconds"] {
                if event["payload"][key].as_i64().filter(|n|*n>=0).is_none() {return Err("Invalid session counter".into());}
            }
            if event["payload"]["active_seconds"].as_i64().unwrap_or(0).saturating_add(event["payload"]["idle_seconds"].as_i64().unwrap_or(0))>60 {return Err("Invalid session interval".into());}
            if !event["payload"]["input_available"].is_boolean() {return Err("Missing input availability".into());}
        }
        self.register_device(event["hostname"].as_str().unwrap_or(device).to_string(),device.to_string(),event["mac_address"].as_str().map(str::to_string),None,event["version"].as_str().map(str::to_string)).await?;
        sqlx::query("INSERT INTO session_events(id,device_id,principal,session_id,kind,timestamp,payload) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
            .bind(id).bind(Uuid::parse_str(device)?).bind(principal).bind(session).bind(kind).bind(timestamp).bind(&event["payload"]).execute(&mut *self.tx).await?;
        Ok(())
    }

    pub async fn update_telemetry_state(&mut self, device: &str, kind: &str,
        observed_at: DateTime<Utc>, payload: serde_json::Value) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO device_telemetry_state(device_id,kind,observed_at,payload) VALUES ($1,$2,$3,$4) ON CONFLICT(device_id,kind) DO UPDATE SET observed_at=EXCLUDED.observed_at, received_at=now(), payload=EXCLUDED.payload WHERE EXCLUDED.observed_at > device_telemetry_state.observed_at")
            .bind(Uuid::parse_str(device).map_err(|error| sqlx::Error::Decode(Box::new(error)))?)
            .bind(kind).bind(observed_at).bind(payload).execute(&mut *self.tx).await?;
        Ok(())
    }

    pub async fn reconcile_inventory(&mut self, device: &str, observed: DateTime<Utc>, payload: &serde_json::Value)
        -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use sha2::{Digest,Sha256};
        let device=Uuid::parse_str(device)?;
        let scope=payload["scope"].as_str().ok_or("Missing inventory scope")?;
        if !matches!(scope,"linux-dpkg"|"windows-machine-64"|"windows-machine-32") { return Err("Unsupported inventory scope".into()); }
        let interval=payload["interval_days"].as_i64().filter(|d|(1..=365).contains(d)).ok_or("Invalid inventory interval")? as i32;
        let complete=payload["complete"].as_bool().ok_or("Missing scan completeness")?;
        let error=payload["error"].as_str().unwrap_or("Source scan failed");
        if error.len()>2048 || payload.to_string().len()>16*1024*1024 {return Err("Oversized inventory snapshot".into());}
        let apps=payload["apps"].as_array().ok_or("Missing inventory entries")?;
        if apps.len()>50000 {return Err("Too many inventory entries".into());}
        let mut entries=std::collections::BTreeMap::new();
        for app in apps {
            let key=app["source_key"].as_str().filter(|v|!v.is_empty()&&v.len()<=1024).ok_or("Invalid inventory key")?;
            let name=app["app_name"].as_str().filter(|v|!v.trim().is_empty()&&v.len()<=2048).ok_or("Invalid inventory name")?;
            let version=app["version"].as_str();
            if (!app["version"].is_null() && version.is_none()) || version.is_some_and(|v|v.len()>1024) {return Err("Invalid inventory version".into());}
            let value=serde_json::json!({"source_key":key,"app_name":name,"version":version});
            if entries.insert(key.to_string(),value).is_some() {return Err("Duplicate inventory key".into());}
        }
        sqlx::query("INSERT INTO inventory_scan_state(device_id,scope,observed_at) VALUES ($1,$2,'1970-01-01 00:00:00+00') ON CONFLICT DO NOTHING")
            .bind(device).bind(scope).execute(&mut *self.tx).await?;
        let (last, previous):(DateTime<Utc>,Option<String>)=sqlx::query_as("SELECT observed_at,snapshot_hash FROM inventory_scan_state WHERE device_id=$1 AND scope=$2 FOR UPDATE")
            .bind(device).bind(scope).fetch_one(&mut *self.tx).await?;
        if observed<=last {return Ok(());}
        if !complete {
            sqlx::query("UPDATE inventory_scan_state SET observed_at=$3,last_error=$4,expected_interval_days=$5 WHERE device_id=$1 AND scope=$2")
                .bind(device).bind(scope).bind(observed).bind(error).bind(interval).execute(&mut *self.tx).await?;
            return Ok(());
        }
        let count=entries.len() as i32;
        let rows=serde_json::Value::Array(entries.into_values().collect());
        let hash=format!("{:x}",Sha256::digest(serde_json::to_vec(&rows)?));
        if previous.as_deref()!=Some(hash.as_str()) {
            // Record differences before replacing current state. Unchanged rows create no history.
            sqlx::query(r#"
                WITH incoming AS (SELECT * FROM jsonb_to_recordset($3) AS x(source_key text,app_name text,version text)),
                existing AS (SELECT * FROM inventory_current WHERE device_id=$1 AND scope=$2)
                INSERT INTO inventory_changes(device_id,scope,source_key,action,before_value,after_value,timestamp)
                SELECT $1,$2,COALESCE(n.source_key,o.source_key),
                    CASE WHEN o.source_key IS NULL THEN 'added' WHEN n.source_key IS NULL THEN 'removed' ELSE 'updated' END,
                    CASE WHEN o.source_key IS NOT NULL THEN jsonb_build_object('app_name',o.app_name,'version',o.version) END,
                    CASE WHEN n.source_key IS NOT NULL THEN jsonb_build_object('app_name',n.app_name,'version',n.version) END,$4
                FROM incoming n FULL JOIN existing o USING(source_key)
                WHERE o.source_key IS NULL OR n.source_key IS NULL OR (o.app_name,o.version) IS DISTINCT FROM (n.app_name,n.version)
            "#).bind(device).bind(scope).bind(&rows).bind(observed).execute(&mut *self.tx).await?;
            sqlx::query(r#"
                INSERT INTO inventory_current(device_id,scope,source_key,app_name,version,changed_at)
                SELECT $1,$2,x.source_key,x.app_name,x.version,$4 FROM jsonb_to_recordset($3) AS x(source_key text,app_name text,version text)
                ON CONFLICT(device_id,scope,source_key) DO UPDATE SET app_name=EXCLUDED.app_name,version=EXCLUDED.version,changed_at=EXCLUDED.changed_at
                WHERE (inventory_current.app_name,inventory_current.version) IS DISTINCT FROM (EXCLUDED.app_name,EXCLUDED.version)
            "#).bind(device).bind(scope).bind(&rows).bind(observed).execute(&mut *self.tx).await?;
            sqlx::query("DELETE FROM inventory_current c WHERE device_id=$1 AND scope=$2 AND NOT EXISTS (SELECT 1 FROM jsonb_to_recordset($3) AS x(source_key text) WHERE x.source_key=c.source_key)")
                .bind(device).bind(scope).bind(&rows).execute(&mut *self.tx).await?;
        }
        sqlx::query("UPDATE inventory_scan_state SET observed_at=$3,last_complete_at=$3,snapshot_hash=$4,last_error=NULL,item_count=$5,expected_interval_days=$6 WHERE device_id=$1 AND scope=$2")
            .bind(device).bind(scope).bind(observed).bind(hash).bind(count).bind(interval).execute(&mut *self.tx).await?;
        Ok(())
    }

    pub async fn begin(pool: &sqlx::PgPool) -> Result<Self, sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout = '5s'").execute(&mut *tx).await?;
        sqlx::query("SET LOCAL statement_timeout = '30s'").execute(&mut *tx).await?;
        Ok(Self { tx })
    }
    pub async fn commit(self) -> Result<(), sqlx::Error> { self.tx.commit().await }
    pub async fn register_processed_event(
        &mut self,
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
        .execute(&mut *self.tx)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn insert_activity_log(
        &mut self,
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
        .execute(&mut *self.tx)
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

    pub async fn insert_security_alert(
        &mut self,
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
        .fetch_one(&mut *self.tx)
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

    pub async fn insert_process_termination_attempt(
        &mut self,
        device_id: Uuid,
        method: &str,
        attempted_by: Option<&str>,
        blocked: bool,
        auto_restarted: bool,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO process_termination_attempts (
                device_id,
                method,
                attempted_by,
                blocked,
                auto_restarted,
                timestamp
            )
            VALUES ($1, $2, $3, $4, $5, COALESCE($6, NOW()))
            "#,
        )
        .bind(device_id)
        .bind(method)
        .bind(attempted_by)
        .bind(blocked)
        .bind(auto_restarted)
        .bind(timestamp)
        .execute(&mut *self.tx)
        .await?;

        Ok(())
    }

    pub async fn insert_node_resource_metric(
        &mut self,
        device_id: String,
        timestamp: DateTime<Utc>,
        cpu_percent: f64,
        memory_used_mb: f64,
        memory_percent: f64,
        top_process_name: Option<String>,
        top_process_cpu_percent: Option<f64>,
        top_process_memory_mb: Option<f64>,
    ) -> Result<(), sqlx::Error> {
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        sqlx::query(
            r#"
            INSERT INTO node_resource_metrics (
                device_id,
                timestamp,
                cpu_percent,
                memory_used_mb,
                memory_percent,
                top_process_name,
                top_process_cpu_percent,
                top_process_memory_mb
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(device_uuid)
        .bind(timestamp)
        .bind(cpu_percent.max(0.0))
        .bind(memory_used_mb.max(0.0))
        .bind(memory_percent.clamp(0.0, 100.0))
        .bind(top_process_name)
        .bind(top_process_cpu_percent.map(|v| v.clamp(0.0, 100.0)))
        .bind(top_process_memory_mb.map(|v| {
            // Backward compatibility: older agent builds sent KB-as-MB values.
            if v > 8192.0 { (v / 1024.0).max(0.0) } else { v.max(0.0) }
        }))
        .execute(&mut *self.tx)
        .await?;

        Ok(())
    }

    pub async fn insert_inventory(
        &mut self,
        device_id: String,
        app_name: String,
        version: String,
        exe_hash: String,
    ) -> Result<InventoryItem, sqlx::Error> {
        let timestamp = Utc::now();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

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
        .bind(device_uuid)
        .bind(&app_name)
        .bind(&version)
        .bind(&exe_hash)
        .bind(timestamp)
        .execute(&mut *self.tx)
        .await?;

        let row = sqlx::query_as::<_, (String, String, String, String, String, DateTime<Utc>)>(
            r#"
            SELECT id::text, device_id::text, app_name, COALESCE(version, ''), COALESCE(exe_hash, ''), timestamp
            FROM inventory
            WHERE device_id = $1
              AND app_name = $2
              AND COALESCE(version, '') = COALESCE($3, '')
              AND COALESCE(exe_hash, '') = COALESCE($4, '')
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(device_uuid)
        .bind(&app_name)
        .bind(&version)
        .bind(&exe_hash)
        .fetch_one(&mut *self.tx)
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

    pub async fn replace_running_apps_snapshot(
        &mut self,
        device_id: String,
        apps: Vec<(String, String, i32, Option<String>, Option<String>)>,
    ) -> Result<(), sqlx::Error> {
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;



        sqlx::query("DELETE FROM running_apps_current WHERE device_id = $1")
            .bind(device_uuid)
            .execute(&mut *self.tx)
            .await?;

        let updated_at = Utc::now();
        for (app_name, primary_title, window_count, exe_path, exe_hash) in apps {
            sqlx::query(
                r#"
                INSERT INTO running_apps_current (id, device_id, app_name, primary_title, window_count, exe_path, exe_hash, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(device_uuid)
            .bind(app_name)
            .bind(primary_title)
            .bind(window_count.max(1))
            .bind(exe_path)
            .bind(exe_hash)
            .bind(updated_at)
            .execute(&mut *self.tx)
            .await?;
        }


        Ok(())
    }

    pub async fn insert_usb_event(
        &mut self,
        device_id: String,
        action: String,
        hardware_id: String,
        device_name: String,
        serial_number: Option<String>,
        volume_label: Option<String>,
        timestamp:DateTime<Utc>,
    ) -> Result<UsbEvent, sqlx::Error> {
        let id = Uuid::new_v4();
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
        .execute(&mut *self.tx)
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

    pub async fn insert_wifi_event(
        &mut self,
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
        .execute(&mut *self.tx)
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

    pub async fn insert_input_summary(
        &mut self,
        device_id: String,
        timestamp: DateTime<Utc>,
        active_seconds: i64,
        idle_seconds: i64,
        keys_count: i64,
        mouse_moves_count: i64,
        clicks_count: i64,
        status: String,
        bucket_max_seconds: i64,
        input_available: Option<bool>,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let safe_bucket_max = bucket_max_seconds.max(1);
        let mut safe_active = active_seconds.max(0).min(safe_bucket_max);
        let mut safe_idle = idle_seconds.max(0).min(safe_bucket_max);
        if safe_active + safe_idle > safe_bucket_max {
            if status == "idle" {
                safe_active = 0;
                safe_idle = safe_bucket_max;
            } else {
                safe_active = safe_bucket_max;
                safe_idle = 0;
            }
        }

        sqlx::query(
            r#"
            INSERT INTO input_activity_metrics (id, device_id, timestamp, active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count, status, input_available)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id)
        .bind(device_uuid)
        .bind(timestamp)
        .bind(safe_active)
        .bind(safe_idle)
        .bind(keys_count.max(0))
        .bind(mouse_moves_count.max(0))
        .bind(clicks_count.max(0))
        .bind(&status)
        .bind(input_available)
        .execute(&mut *self.tx)
        .await?;

        Ok(())
    }

    pub async fn register_device(
        &mut self,
        hostname: String,
        device_id: String,
        mac_address: Option<String>,
        nickname: Option<String>,
        version: Option<String>,
    ) -> Result<Device, sqlx::Error> {
        let last_seen = Utc::now();
        let device_uuid = Uuid::parse_str(&device_id)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        sqlx::query(
            r#"
            INSERT INTO devices (device_id, hostname, nickname, mac_address, version, last_seen)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (device_id) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                nickname = COALESCE(EXCLUDED.nickname, devices.nickname),
                mac_address = COALESCE(EXCLUDED.mac_address, devices.mac_address),
                version = COALESCE(EXCLUDED.version, devices.version),
                last_seen = EXCLUDED.last_seen,
                updated_at = NOW()
            "#,
        )
        .bind(device_uuid)
        .bind(&hostname)
        .bind(&nickname)
        .bind(&mac_address)
        .bind(&version)
        .bind(last_seen)
        .execute(&mut *self.tx)
        .await?;

        tracing::info!("🖥️  Registered device: {} ({})", hostname, device_id);

        Ok(Device {
            id: device_uuid,
            hostname,
            device_id: device_uuid,
            mac_address,
            version,
            created_at: last_seen,
            last_seen,
            nickname,
        })
    }

    pub async fn insert_security_event(
        &mut self,
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
        .fetch_optional(&mut *self.tx)
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
        .fetch_one(&mut *self.tx)
        .await
        .map(|(id, ts, did, qn, qp, mt, sev, rd, fp, ca)| SecurityEvent {
            id, timestamp: ts, device_id: did,
            query_name: qn, query_pack: qp, mitre_technique: mt,
            severity: sev, raw_data: rd, event_fingerprint: fp, created_at: ca,
        })
    }
}
