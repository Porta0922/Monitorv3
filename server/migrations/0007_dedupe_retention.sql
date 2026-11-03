CREATE INDEX IF NOT EXISTS processed_events_retention ON processed_events(received_at, event_id);
