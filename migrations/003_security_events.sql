-- Migration 003: security_events table for osquery + MITRE ATT&CK integration

CREATE TABLE IF NOT EXISTS security_events (
    id              BIGSERIAL PRIMARY KEY,
    timestamp       TIMESTAMPTZ NOT NULL,
    device_id       UUID        NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    query_name      TEXT        NOT NULL,
    query_pack      TEXT,
    mitre_technique VARCHAR(20),
    severity        VARCHAR(20) NOT NULL,
    raw_data        JSONB       NOT NULL DEFAULT '{}',
    event_fingerprint VARCHAR(128),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_security_events_device_ts
    ON security_events(device_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_security_events_timestamp
    ON security_events(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_security_events_severity
    ON security_events(severity);

CREATE INDEX IF NOT EXISTS idx_security_events_technique
    ON security_events(mitre_technique);

CREATE UNIQUE INDEX IF NOT EXISTS idx_security_events_fingerprint
    ON security_events(event_fingerprint)
    WHERE event_fingerprint IS NOT NULL;
