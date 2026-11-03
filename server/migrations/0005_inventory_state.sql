CREATE TABLE inventory_scan_state (
    device_id uuid NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    scope text NOT NULL,
    observed_at timestamptz NOT NULL,
    last_complete_at timestamptz,
    snapshot_hash text,
    last_error text,
    item_count integer,
    expected_interval_days integer NOT NULL DEFAULT 30 CHECK(expected_interval_days BETWEEN 1 AND 365),
    PRIMARY KEY(device_id,scope)
);
CREATE TABLE inventory_current (
    device_id uuid NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    scope text NOT NULL,
    source_key text NOT NULL,
    app_name text NOT NULL,
    version text,
    changed_at timestamptz NOT NULL,
    PRIMARY KEY(device_id,scope,source_key)
);
CREATE TABLE inventory_changes (
    id bigserial PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    scope text NOT NULL,
    source_key text NOT NULL,
    action text NOT NULL CHECK(action IN ('added','updated','removed')),
    before_value jsonb,
    after_value jsonb,
    timestamp timestamptz NOT NULL
);
CREATE INDEX inventory_changes_device_time ON inventory_changes(device_id,timestamp DESC,id DESC);
CREATE INDEX inventory_changes_retention ON inventory_changes(timestamp,id);
-- Legacy inventory remains historical observations; do not infer current installation from it.
