-- Holds protect existing and late-arriving events in an inclusive UTC interval.
CREATE TABLE security_investigations (
    id uuid PRIMARY KEY,
    device_id uuid NOT NULL REFERENCES devices(device_id),
    title text NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
    reason text NOT NULL CHECK (length(reason) BETWEEN 1 AND 4000),
    event_from timestamptz NOT NULL,
    event_to timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    released_at timestamptz,
    release_reason text,
    CHECK (event_to >= event_from)
);
CREATE INDEX security_investigation_active ON security_investigations(device_id, event_from, event_to) WHERE released_at IS NULL;
CREATE INDEX security_events_retention ON security_events(timestamp, id);
