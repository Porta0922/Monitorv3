\set ON_ERROR_STOP on
BEGIN;
SET LOCAL timezone = 'UTC';
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '120s';
SELECT set_config('maintenance.job', :'job', true),
       set_config('maintenance.days', :'retention_days', true),
       set_config('maintenance.batch', :'batch_size', true);
SELECT pg_try_advisory_xact_lock(734203401) AS acquired \gset
\if :acquired
DO $$
DECLARE
    job_name text := current_setting('maintenance.job');
    days integer := current_setting('maintenance.days')::integer;
    batch integer := current_setting('maintenance.batch')::integer;
    table_name text;
    time_column text := 'timestamp';
    identity_column text := 'id';
    predicate text := 'TRUE';
    cutoff timestamptz;
    removed bigint;
    remaining bigint;
    oldest timestamptz;
BEGIN
    IF days < 1 OR days > 3650 OR batch < 1 OR batch > 100000 THEN
        RAISE EXCEPTION 'Invalid retention settings';
    END IF;
    CASE job_name
        WHEN 'deduplication' THEN
            IF days < 32 THEN RAISE EXCEPTION 'Dedupe retention must be at least 32 days'; END IF;
            table_name := 'processed_events'; time_column := 'received_at'; identity_column := 'event_id';
        WHEN 'security' THEN
            table_name := 'security_events';
            predicate := 'NOT EXISTS (SELECT 1 FROM public.security_investigations h WHERE h.device_id=t.device_id AND h.released_at IS NULL AND t.timestamp BETWEEN h.event_from AND h.event_to)';
        WHEN 'inventory-changes' THEN table_name := 'inventory_changes';
        WHEN 'activity' THEN table_name := 'activity_logs';
        WHEN 'sessions' THEN table_name := 'session_events';
        WHEN 'usb' THEN table_name := 'usb_events';
        WHEN 'wifi' THEN table_name := 'wifi_events';
        WHEN 'audit' THEN table_name := 'audit_events'; time_column := 'created_at';
        WHEN 'resolved-alerts' THEN
            table_name := 'security_alerts'; time_column := 'resolved_at';
            predicate := 't.resolved IS TRUE AND t.resolved_at IS NOT NULL';
        ELSE RAISE EXCEPTION 'Unknown history job';
    END CASE;
    cutoff := date_trunc('day', now()) - make_interval(days => days);
    INSERT INTO public.maintenance_job_status(job, retention_days, batch_limit, last_attempt_at)
    VALUES (job_name, days, batch, clock_timestamp())
    ON CONFLICT (job) DO UPDATE SET retention_days = EXCLUDED.retention_days,
        batch_limit = EXCLUDED.batch_limit, last_attempt_at = EXCLUDED.last_attempt_at;
    BEGIN
        IF job_name='sessions' THEN
          WITH candidates AS (SELECT id FROM session_events WHERE timestamp<cutoff ORDER BY timestamp,id LIMIT batch FOR UPDATE SKIP LOCKED),
          moved AS (DELETE FROM session_events e USING candidates c WHERE e.id=c.id RETURNING e.*),
          compacted AS (
           INSERT INTO session_activity_daily(device_id,day,principal,session_id,first_seen,last_seen,events,available_samples,unavailable_samples,unknown_samples,active_person_seconds,idle_person_seconds,keys_count,mouse_moves_count,clicks_count)
                   SELECT device_id,(timestamp AT TIME ZONE 'UTC')::date AS day,principal,session_id,min(timestamp) AS first_seen,max(timestamp) AS last_seen,count(*) AS events,
          count(*) FILTER(WHERE kind='input_summary' AND payload->>'input_available'='true') AS available_samples,
          count(*) FILTER(WHERE kind='input_summary' AND payload->>'input_available'='false') AS unavailable_samples,
          count(*) FILTER(WHERE kind='input_summary' AND COALESCE(payload->>'input_available','') NOT IN ('true','false')) AS unknown_samples,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'active_seconds')::bigint END) AS active_person_seconds,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'idle_seconds')::bigint END) AS idle_person_seconds,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'keys_count')::bigint END) AS keys_count,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'mouse_moves_count')::bigint END) AS mouse_moves_count,
          sum(CASE WHEN kind='input_summary' AND payload->>'input_available'='true' THEN (payload->>'clicks_count')::bigint END) AS clicks_count
 FROM moved GROUP BY device_id,(timestamp AT TIME ZONE 'UTC')::date,principal,session_id
           ON CONFLICT(device_id,day,principal,session_id) DO UPDATE SET
            first_seen=LEAST(session_activity_daily.first_seen,excluded.first_seen),last_seen=GREATEST(session_activity_daily.last_seen,excluded.last_seen),events=COALESCE(session_activity_daily.events,0)+COALESCE(excluded.events,0),available_samples=COALESCE(session_activity_daily.available_samples,0)+COALESCE(excluded.available_samples,0),unavailable_samples=COALESCE(session_activity_daily.unavailable_samples,0)+COALESCE(excluded.unavailable_samples,0),unknown_samples=COALESCE(session_activity_daily.unknown_samples,0)+COALESCE(excluded.unknown_samples,0),active_person_seconds=CASE WHEN session_activity_daily.active_person_seconds IS NULL AND excluded.active_person_seconds IS NULL THEN NULL ELSE COALESCE(session_activity_daily.active_person_seconds,0)+COALESCE(excluded.active_person_seconds,0) END,idle_person_seconds=CASE WHEN session_activity_daily.idle_person_seconds IS NULL AND excluded.idle_person_seconds IS NULL THEN NULL ELSE COALESCE(session_activity_daily.idle_person_seconds,0)+COALESCE(excluded.idle_person_seconds,0) END,keys_count=CASE WHEN session_activity_daily.keys_count IS NULL AND excluded.keys_count IS NULL THEN NULL ELSE COALESCE(session_activity_daily.keys_count,0)+COALESCE(excluded.keys_count,0) END,mouse_moves_count=CASE WHEN session_activity_daily.mouse_moves_count IS NULL AND excluded.mouse_moves_count IS NULL THEN NULL ELSE COALESCE(session_activity_daily.mouse_moves_count,0)+COALESCE(excluded.mouse_moves_count,0) END,clicks_count=CASE WHEN session_activity_daily.clicks_count IS NULL AND excluded.clicks_count IS NULL THEN NULL ELSE COALESCE(session_activity_daily.clicks_count,0)+COALESCE(excluded.clicks_count,0) END
           RETURNING device_id
          ) SELECT count(*) INTO removed FROM moved;
          -- Keep compacted session counters for at most one further year; raw detail
          -- and its configured retention remain independent of resource counters.
          DELETE FROM session_activity_daily WHERE (device_id,day,principal,session_id) IN
           (SELECT device_id,day,principal,session_id FROM session_activity_daily WHERE day<current_date-(days+365) ORDER BY day LIMIT batch);
        ELSE
        -- Match logical identity and timestamp; do not depend on compressed ctids.
        EXECUTE format('WITH candidates AS (
            SELECT t.%4$I AS identity_value, t.%1$I AS event_time FROM public.%2$I t
            WHERE t.%1$I < $1 AND (%3$s)
            ORDER BY t.%1$I, t.%4$I LIMIT $2 FOR UPDATE SKIP LOCKED
        ) DELETE FROM public.%2$I t USING candidates c
          WHERE t.%4$I = c.identity_value AND t.%1$I = c.event_time AND (%3$s)',
          time_column, table_name, predicate, identity_column) USING cutoff, batch;
        GET DIAGNOSTICS removed = ROW_COUNT;

        END IF;
        IF removed > batch THEN
            RAISE EXCEPTION 'Non-unique legacy identities exceeded batch limit';
        END IF;
        EXECUTE format('SELECT count(*), min(event_time) FROM (
            SELECT t.%1$I AS event_time FROM public.%2$I t
            WHERE t.%1$I < $1 AND (%3$s)
            ORDER BY t.%1$I LIMIT $2) pending', time_column, table_name, predicate)
            INTO remaining, oldest USING cutoff, batch + 1;
        UPDATE public.maintenance_job_status SET last_success_at = clock_timestamp(),
            last_error = NULL, last_rows_processed = removed,
            pending_rows_lower_bound = remaining, backlog_truncated = remaining > batch,
            oldest_pending_at = oldest WHERE job = job_name;
    EXCEPTION WHEN OTHERS OR query_canceled THEN
        -- This block rolls back deletions, but retains a diagnostic for the operator.
        UPDATE public.maintenance_job_status SET last_failure_at = clock_timestamp(),
            last_error = SQLSTATE || ': ' || SQLERRM, last_rows_processed = 0,
            pending_rows_lower_bound = NULL, backlog_truncated = NULL,
            oldest_pending_at = NULL WHERE job = job_name;
    END;
END $$;
SELECT last_error IS NOT NULL AS failed FROM public.maintenance_job_status
WHERE job = :'job' \gset
COMMIT;
\if :failed
DO $$ BEGIN RAISE EXCEPTION 'History maintenance failed; inspect maintenance_job_status'; END $$;
\endif
\else
ROLLBACK;
\echo 'History maintenance skipped: another maintenance transaction holds the lock'
\endif
