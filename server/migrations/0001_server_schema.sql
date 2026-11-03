SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '120s';

-- Canonical server schema. Applied once, transactionally, by SQLx.
-- Compatible with empty databases and legacy installations.

CREATE TABLE IF NOT EXISTS devices (
                device_id UUID PRIMARY KEY,
                hostname VARCHAR(255) NOT NULL,
                nickname VARCHAR(255),
                mac_address VARCHAR(17),
                version VARCHAR(64),
                last_seen TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE TABLE IF NOT EXISTS running_apps_current (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                app_name VARCHAR(255) NOT NULL,
                primary_title TEXT,
                window_count INTEGER NOT NULL DEFAULT 1,
                exe_path TEXT,
                exe_hash VARCHAR(255),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_running_apps_current_device_updated_at ON running_apps_current(device_id, updated_at DESC);

ALTER TABLE devices ADD COLUMN IF NOT EXISTS nickname VARCHAR(255);

ALTER TABLE devices ADD COLUMN IF NOT EXISTS mac_address VARCHAR(17);

ALTER TABLE devices ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE devices ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE devices ADD COLUMN IF NOT EXISTS last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE devices ADD COLUMN IF NOT EXISTS version VARCHAR(64);

CREATE TABLE IF NOT EXISTS agent_commands (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
                command VARCHAR(64) NOT NULL,
                payload JSONB NOT NULL DEFAULT '{}'::jsonb,
                status VARCHAR(32) NOT NULL DEFAULT 'pending',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                delivered_at TIMESTAMPTZ,
                acked_at TIMESTAMPTZ,
                result JSONB
            );

CREATE INDEX IF NOT EXISTS idx_agent_commands_pending ON agent_commands(device_id, status);

CREATE TABLE IF NOT EXISTS activity_logs (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                app_name VARCHAR(255) NOT NULL,
                window_title TEXT,
                duration_seconds BIGINT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE activity_logs ADD COLUMN IF NOT EXISTS id UUID;

UPDATE activity_logs SET id = gen_random_uuid() WHERE id IS NULL;

CREATE TABLE IF NOT EXISTS inventory (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                app_name VARCHAR(255) NOT NULL,
                version VARCHAR(255),
                exe_hash VARCHAR(255),
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

-- Avoid rewriting UUID columns or changing existing foreign-key semantics.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
               WHERE table_schema='public' AND table_name='activity_logs'
               AND column_name='device_id' AND data_type <> 'uuid') THEN
        ALTER TABLE activity_logs DROP CONSTRAINT IF EXISTS activity_logs_device_id_fkey;
        ALTER TABLE activity_logs ALTER COLUMN device_id TYPE UUID USING device_id::uuid;
        ALTER TABLE activity_logs ADD CONSTRAINT activity_logs_device_id_fkey
            FOREIGN KEY (device_id) REFERENCES devices(device_id);
    END IF;
END $$;

-- Avoid rewriting UUID columns or changing existing foreign-key semantics.
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
               WHERE table_schema='public' AND table_name='inventory'
               AND column_name='device_id' AND data_type <> 'uuid') THEN
        ALTER TABLE inventory DROP CONSTRAINT IF EXISTS inventory_device_id_fkey;
        ALTER TABLE inventory ALTER COLUMN device_id TYPE UUID USING device_id::uuid;
        ALTER TABLE inventory ADD CONSTRAINT inventory_device_id_fkey
            FOREIGN KEY (device_id) REFERENCES devices(device_id);
    END IF;
END $$;

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
            );

CREATE INDEX IF NOT EXISTS idx_usb_events_device_timestamp ON usb_events(device_id, timestamp DESC);

CREATE TABLE IF NOT EXISTS wifi_events (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                interface_name VARCHAR(255) NOT NULL,
                state VARCHAR(32) NOT NULL,
                ssid VARCHAR(255),
                bssid VARCHAR(64),
                signal_percent INTEGER,
                timestamp TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_wifi_events_device_timestamp ON wifi_events(device_id, timestamp DESC);

CREATE TABLE IF NOT EXISTS input_activity_metrics (
                id UUID PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                timestamp TIMESTAMPTZ NOT NULL,
                active_seconds BIGINT NOT NULL DEFAULT 0,
                idle_seconds BIGINT NOT NULL DEFAULT 0,
                keys_count BIGINT NOT NULL DEFAULT 0,
                mouse_moves_count BIGINT NOT NULL DEFAULT 0,
                clicks_count BIGINT NOT NULL DEFAULT 0,
                status VARCHAR(16) NOT NULL DEFAULT 'active',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_input_activity_metrics_device_timestamp ON input_activity_metrics(device_id, timestamp DESC);

ALTER TABLE input_activity_metrics ADD COLUMN IF NOT EXISTS keys_count BIGINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_input_activity_metrics_timestamp ON input_activity_metrics(timestamp);

ALTER TABLE input_activity_metrics ADD COLUMN IF NOT EXISTS mouse_moves_count BIGINT NOT NULL DEFAULT 0;

ALTER TABLE input_activity_metrics ADD COLUMN IF NOT EXISTS clicks_count BIGINT NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS node_resource_metrics (
                id BIGSERIAL PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                timestamp TIMESTAMPTZ NOT NULL,
                cpu_percent DOUBLE PRECISION NOT NULL DEFAULT 0,
                memory_used_mb DOUBLE PRECISION NOT NULL DEFAULT 0,
                memory_percent DOUBLE PRECISION NOT NULL DEFAULT 0,
                top_process_name TEXT,
                top_process_cpu_percent DOUBLE PRECISION,
                top_process_memory_mb DOUBLE PRECISION,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_node_resource_metrics_device_timestamp ON node_resource_metrics(device_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_node_resource_metrics_timestamp ON node_resource_metrics(timestamp DESC);

CREATE TABLE IF NOT EXISTS processed_events (
                event_id VARCHAR(64) PRIMARY KEY,
                device_id VARCHAR(255) NOT NULL,
                event_type VARCHAR(64) NOT NULL,
                sequence BIGINT,
                boot_id VARCHAR(64),
                received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_processed_events_device_received_at ON processed_events(device_id, received_at DESC);

CREATE TABLE IF NOT EXISTS audit_events (
                id BIGSERIAL PRIMARY KEY,
                actor VARCHAR(255) NOT NULL,
                action VARCHAR(128) NOT NULL,
                target VARCHAR(255) NOT NULL,
                details TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_audit_events_created_at ON audit_events(created_at DESC);

CREATE TABLE IF NOT EXISTS security_alerts (
                id BIGSERIAL PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                alert_type VARCHAR(64) NOT NULL,
                app_name VARCHAR(255) NOT NULL DEFAULT '',
                exe_hash VARCHAR(255) NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                severity VARCHAR(16) NOT NULL,
                resolved BOOLEAN NOT NULL DEFAULT FALSE,
                resolution_notes TEXT,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_security_alerts_device_created ON security_alerts(device_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_security_alerts_resolved ON security_alerts(resolved, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_security_alerts_severity ON security_alerts(severity, created_at DESC);

ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS resolution_notes TEXT;

ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS resolved_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS process_termination_attempts (
                id BIGSERIAL PRIMARY KEY,
                device_id UUID NOT NULL REFERENCES devices(device_id),
                method VARCHAR(128) NOT NULL,
                attempted_by VARCHAR(255),
                blocked BOOLEAN NOT NULL DEFAULT TRUE,
                auto_restarted BOOLEAN NOT NULL DEFAULT TRUE,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_process_termination_attempts_device_ts ON process_termination_attempts(device_id, timestamp DESC);

CREATE TABLE IF NOT EXISTS security_events (
                id                BIGSERIAL PRIMARY KEY,
                timestamp         TIMESTAMPTZ NOT NULL,
                device_id         UUID        NOT NULL REFERENCES devices(device_id),
                query_name        TEXT        NOT NULL,
                query_pack        TEXT,
                mitre_technique   VARCHAR(20),
                severity          VARCHAR(20) NOT NULL,
                raw_data          JSONB       NOT NULL DEFAULT '{}',
                event_fingerprint VARCHAR(128),
                created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

CREATE INDEX IF NOT EXISTS idx_security_events_device_ts ON security_events(device_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_security_events_timestamp ON security_events(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_security_events_severity ON security_events(severity);

CREATE INDEX IF NOT EXISTS idx_security_events_technique ON security_events(mitre_technique);

CREATE UNIQUE INDEX IF NOT EXISTS idx_security_events_fingerprint ON security_events(event_fingerprint) WHERE event_fingerprint IS NOT NULL;

-- Reconcile columns missing from the legacy Docker bootstrap.
ALTER TABLE activity_logs ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ;
UPDATE activity_logs SET created_at = timestamp WHERE created_at IS NULL;
ALTER TABLE activity_logs ALTER COLUMN created_at SET DEFAULT NOW();
ALTER TABLE activity_logs ALTER COLUMN created_at SET NOT NULL;
ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE process_termination_attempts ADD COLUMN IF NOT EXISTS auto_restarted BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE security_alerts ALTER COLUMN id TYPE BIGINT;
