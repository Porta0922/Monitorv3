-- Bounded current state, not a new unbounded telemetry history.
CREATE TABLE device_telemetry_state (
    device_id UUID NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('resources', 'health')),
    observed_at TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    payload JSONB NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    PRIMARY KEY (device_id, kind)
);
