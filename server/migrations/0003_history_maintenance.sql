-- Status is committed separately from the guarded deletion subtransaction.
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '120s';
CREATE TABLE public.maintenance_job_status (
    job text PRIMARY KEY,
    retention_days integer NOT NULL,
    batch_limit integer NOT NULL,
    last_attempt_at timestamptz NOT NULL,
    last_success_at timestamptz,
    last_failure_at timestamptz,
    last_error text,
    last_rows_processed bigint NOT NULL DEFAULT 0,
    pending_rows_lower_bound bigint,
    backlog_truncated boolean,
    oldest_pending_at timestamptz
);
CREATE INDEX IF NOT EXISTS idx_usb_events_retention ON public.usb_events(timestamp, id);
CREATE INDEX IF NOT EXISTS idx_wifi_events_retention ON public.wifi_events(timestamp, id);
CREATE INDEX IF NOT EXISTS idx_security_alerts_retention ON public.security_alerts(resolved_at, id)
    WHERE resolved IS TRUE AND resolved_at IS NOT NULL;
