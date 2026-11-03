-- New multi-principal events are deliberately separate from machine-duration totals.
CREATE TABLE session_events (
 id uuid PRIMARY KEY, device_id uuid NOT NULL REFERENCES devices(device_id),
 principal text NOT NULL, session_id uuid NOT NULL, kind text NOT NULL,
 timestamp timestamptz NOT NULL, payload jsonb NOT NULL
);
CREATE INDEX session_events_device_time ON session_events(device_id,timestamp DESC,id DESC);
CREATE INDEX session_events_scope_time ON session_events(device_id,principal,session_id,timestamp DESC);
CREATE INDEX session_events_retention ON session_events(timestamp,id);
