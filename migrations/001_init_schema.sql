-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- 1. Devices Table (Master record of monitored machines)
CREATE TABLE IF NOT EXISTS devices (
    device_id UUID PRIMARY KEY,
    hostname VARCHAR(255) NOT NULL,
    nickname VARCHAR(255),
    mac_address VARCHAR(17),
    last_seen TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_devices_hostname ON devices(hostname);
CREATE INDEX idx_devices_last_seen ON devices(last_seen DESC);

-- 2. Activity Logs Hypertable (Time-series data with 1-day partitioning)
CREATE TABLE IF NOT EXISTS activity_logs (
    timestamp TIMESTAMPTZ NOT NULL,
    device_id UUID NOT NULL,
    app_name VARCHAR(255),
    window_title TEXT,
    duration_seconds BIGINT,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

-- Convert to hypertable with 1-day intervals
SELECT create_hypertable(
    'activity_logs',
    'timestamp',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '1 day'
);

-- Compression policy (compress chunks older than 30 days)
ALTER TABLE activity_logs SET (timescaledb.compress, timescaledb.compress_segmentby = 'device_id');
SELECT add_compression_policy('activity_logs', INTERVAL '30 days', if_not_exists => true);

-- Retention policy (keep 90 days of detail, then rollup)
SELECT add_retention_policy('activity_logs', INTERVAL '90 days', if_not_exists => true);

-- Indices for common queries
CREATE INDEX idx_activity_logs_device_timestamp ON activity_logs(device_id, timestamp DESC);
CREATE INDEX idx_activity_logs_app_name ON activity_logs(app_name);

-- 3. Software Inventory Table
CREATE TABLE IF NOT EXISTS app_inventory (
    id SERIAL PRIMARY KEY,
    device_id UUID NOT NULL,
    app_name VARCHAR(255) NOT NULL,
    version VARCHAR(50),
    exe_hash VARCHAR(64) NOT NULL UNIQUE,
    verified BOOLEAN DEFAULT FALSE,
    last_detected TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE,
    UNIQUE(device_id, app_name, exe_hash)
);

CREATE INDEX idx_app_inventory_device ON app_inventory(device_id);
CREATE INDEX idx_app_inventory_hash ON app_inventory(exe_hash);

-- 4. Software Whitelist (Global hash registry)
CREATE TABLE IF NOT EXISTS software_whitelist (
    id SERIAL PRIMARY KEY,
    app_name VARCHAR(255) NOT NULL,
    exe_hash VARCHAR(64) NOT NULL UNIQUE,
    version VARCHAR(50),
    is_approved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_whitelist_hash ON software_whitelist(exe_hash);
CREATE INDEX idx_whitelist_app_name ON software_whitelist(app_name);

-- 5. USB/External Device History
CREATE TABLE IF NOT EXISTS usb_history (
    timestamp TIMESTAMPTZ NOT NULL,
    device_id UUID NOT NULL,
    action VARCHAR(10) NOT NULL, -- 'IN' for connected, 'OUT' for disconnected
    hardware_id VARCHAR(255) NOT NULL,
    vendor_id VARCHAR(10),
    product_id VARCHAR(10),
    serial_number VARCHAR(255),
    device_name VARCHAR(255),
    volume_label VARCHAR(255),
    capacity_bytes BIGINT,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

-- USB history is also time-series, can be hypertable
SELECT create_hypertable(
    'usb_history',
    'timestamp',
    if_not_exists => TRUE,
    chunk_time_interval => INTERVAL '7 days'
);

CREATE INDEX idx_usb_history_device ON usb_history(device_id, timestamp DESC);
CREATE INDEX idx_usb_history_action ON usb_history(action);
CREATE INDEX idx_usb_history_hardware ON usb_history(hardware_id);

-- 6. Security Alerts Table
CREATE TABLE IF NOT EXISTS security_alerts (
    id SERIAL PRIMARY KEY,
    device_id UUID NOT NULL,
    alert_type VARCHAR(50),
    app_name VARCHAR(255),
    exe_hash VARCHAR(64),
    description TEXT,
    severity VARCHAR(20), -- 'LOW', 'MEDIUM', 'HIGH', 'CRITICAL'
    resolved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

CREATE INDEX idx_security_alerts_device ON security_alerts(device_id, created_at DESC);
CREATE INDEX idx_security_alerts_severity ON security_alerts(severity);

-- 7. Users Table (Admin authentication)
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    is_admin BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- Seed default admin user (password: 'admin' - Argon2id hashed, change in production!)
-- INSERT INTO users (username, email, password_hash, is_admin) 
-- VALUES ('admin', 'admin@example.com', '<hash>', TRUE);

COMMIT;
