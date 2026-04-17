-- Migration: Add Input Activity Heatmaps and Enhanced Security Alerts

-- 1. Input Activity Heatmaps Table (Mouse/Keyboard activity by coordinate)
CREATE TABLE IF NOT EXISTS input_activity_heatmaps (
    id BIGSERIAL NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    device_id UUID NOT NULL,
    
    -- Grid data (100x100 pixel grid, stores click/movement density)
    grid_data JSONB NOT NULL,  -- Format: {(x,y): count, ...}
    
    -- Metadata
    screen_width INTEGER,      -- Screen resolution width
    screen_height INTEGER,     -- Screen resolution height
    total_mouse_moves INTEGER,
    total_mouse_clicks INTEGER,
    total_keyboard_events INTEGER,
    
    -- Tracking
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (timestamp, id),
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

-- Create hypertable for heatmaps (7-day chunks for faster queries)
SELECT create_hypertable(
    'input_activity_heatmaps',
    'timestamp',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '7 days'
);

-- Compress heatmaps older than 60 days
ALTER TABLE input_activity_heatmaps SET (timescaledb.compress, timescaledb.compress_segmentby = 'device_id');
SELECT add_compression_policy('input_activity_heatmaps', INTERVAL '60 days', if_not_exists => true);

-- Retention: Keep 180 days of heatmap data
SELECT add_retention_policy('input_activity_heatmaps', INTERVAL '180 days', if_not_exists => true);

-- Indices for heatmap queries
CREATE INDEX idx_heatmaps_device_timestamp ON input_activity_heatmaps(device_id, timestamp DESC);
CREATE INDEX idx_heatmaps_timestamp ON input_activity_heatmaps(timestamp DESC);

-- 2. Enhanced Security Alerts Table
CREATE TABLE IF NOT EXISTS security_alerts (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID NOT NULL,
    alert_type VARCHAR(50),
    app_name VARCHAR(255),
    exe_hash VARCHAR(64),
    description TEXT,
    severity VARCHAR(20),
    resolved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS resolution_notes TEXT;
ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();
ALTER TABLE security_alerts ADD COLUMN IF NOT EXISTS timestamp TIMESTAMPTZ DEFAULT NOW();

-- Keep security_alerts as a regular table for compatibility with the base schema.
CREATE INDEX IF NOT EXISTS idx_alerts_device_timestamp ON security_alerts(device_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_alerts_type ON security_alerts(alert_type);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON security_alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_resolved ON security_alerts(resolved);

-- 3. Process Protection Event Logs
CREATE TABLE IF NOT EXISTS process_termination_attempts (
    id BIGSERIAL NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    device_id UUID NOT NULL,
    
    -- Termination attempt details
    method VARCHAR(50),                -- taskkill, kill, killall, Process.Kill(), etc
    attempted_by VARCHAR(255),         -- Username or system account
    process_id INTEGER,                -- PID that tried to kill
    command_line TEXT,                 -- Full command used
    
    -- Response
    blocked BOOLEAN DEFAULT TRUE,      -- Was it blocked?
    action_taken VARCHAR(100),         -- auto-restart, alert, etc
    
    -- Tracking
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (timestamp, id),
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

-- Create hypertable for process events (1-day chunks)
SELECT create_hypertable(
    'process_termination_attempts',
    'timestamp',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '1 day'
);

-- Compress after 30 days
ALTER TABLE process_termination_attempts SET (timescaledb.compress, timescaledb.compress_segmentby = 'device_id');
SELECT add_compression_policy('process_termination_attempts', INTERVAL '30 days', if_not_exists => true);

-- Retention: Keep 90 days of process events
SELECT add_retention_policy('process_termination_attempts', INTERVAL '90 days', if_not_exists => true);

-- Indices
CREATE INDEX idx_process_events_device_timestamp ON process_termination_attempts(device_id, timestamp DESC);
CREATE INDEX idx_process_events_blocked ON process_termination_attempts(blocked);

-- 4. Input Activity Daily Summary (for dashboard heatmaps)
CREATE TABLE IF NOT EXISTS input_activity_daily_summary (
    date DATE NOT NULL,
    device_id UUID NOT NULL,
    
    -- Aggregated activity
    total_mouse_moves BIGINT,
    total_mouse_clicks BIGINT,
    total_keyboard_events BIGINT,
    
    -- Peak hours (JSON with hour: activity_count)
    peak_hours JSONB,
    
    -- Average heatmap (aggregated over the day)
    average_heatmap JSONB,
    
    -- Tracking
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    PRIMARY KEY (date, device_id),
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

CREATE INDEX idx_daily_summary_device_date ON input_activity_daily_summary(device_id, date DESC);

-- Grants for monitor_user (if exists)
DO $$
BEGIN
    IF EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'monitor_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON input_activity_heatmaps TO monitor_user;
        GRANT SELECT, INSERT, UPDATE ON security_alerts TO monitor_user;
        GRANT SELECT, INSERT ON process_termination_attempts TO monitor_user;
        GRANT SELECT, INSERT, UPDATE ON input_activity_daily_summary TO monitor_user;
    END IF;
END $$;
