\set ON_ERROR_STOP on
BEGIN;
SET LOCAL TIME ZONE 'UTC';
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '120s';
SELECT set_config('monitor.raw_days', :'raw_days', true),
       set_config('monitor.summary_days', :'summary_days', true),
       set_config('monitor.batch_size', :'batch_size', true);
SELECT pg_try_advisory_xact_lock(734203401) AS acquired \gset
\if :acquired
DO $$
DECLARE
    raw_days integer := current_setting('monitor.raw_days')::integer;
    summary_days integer := current_setting('monitor.summary_days')::integer;
    batch_size integer := current_setting('monitor.batch_size')::integer;
    raw_cutoff timestamptz;
    summary_cutoff date;
    resource_count bigint;
    input_count bigint;
BEGIN
    IF raw_days < 1 OR summary_days <= raw_days OR summary_days > 3650
       OR batch_size < 1 OR batch_size > 100000 THEN
        RAISE EXCEPTION 'Invalid retention configuration';
    END IF;
    raw_cutoff := date_trunc('day', now()) - make_interval(days => raw_days);
    summary_cutoff := current_date - summary_days;
    -- DELETE RETURNING and aggregation are atomic. A failed aggregate rolls
    -- back deletion. Retrying or receiving late samples cannot double-count.
    WITH candidates AS (
        SELECT tableoid, ctid FROM public.node_resource_metrics
        WHERE timestamp < raw_cutoff
        ORDER BY timestamp LIMIT batch_size FOR UPDATE SKIP LOCKED
    ), moved AS (
        DELETE FROM public.node_resource_metrics t USING candidates c
        WHERE t.tableoid = c.tableoid AND t.ctid = c.ctid RETURNING t.*
    ), aggregated AS (
        INSERT INTO public.resource_metrics_daily AS d
        SELECT device_id, (timestamp AT TIME ZONE 'UTC')::date,
               count(*), sum(cpu_percent), max(cpu_percent),
               sum(memory_percent), max(memory_percent), sum(memory_used_mb)
        FROM moved WHERE (timestamp AT TIME ZONE 'UTC')::date >= summary_cutoff
        GROUP BY device_id, (timestamp AT TIME ZONE 'UTC')::date
        ON CONFLICT (day, device_id) DO UPDATE SET
            samples = d.samples + EXCLUDED.samples,
            cpu_sum = d.cpu_sum + EXCLUDED.cpu_sum,
            cpu_max = greatest(d.cpu_max, EXCLUDED.cpu_max),
            memory_percent_sum = d.memory_percent_sum + EXCLUDED.memory_percent_sum,
            memory_percent_max = greatest(d.memory_percent_max, EXCLUDED.memory_percent_max),
            memory_used_mb_sum = d.memory_used_mb_sum + EXCLUDED.memory_used_mb_sum
        RETURNING 1
    ) SELECT count(*) INTO resource_count FROM moved;
    WITH candidates AS (
        SELECT tableoid, ctid FROM public.input_activity_metrics
        WHERE timestamp < raw_cutoff
        ORDER BY timestamp LIMIT batch_size FOR UPDATE SKIP LOCKED
    ), moved AS (
        DELETE FROM public.input_activity_metrics t USING candidates c
        WHERE t.tableoid = c.tableoid AND t.ctid = c.ctid RETURNING t.*
    ), aggregated AS (
        INSERT INTO public.input_metrics_daily AS d (device_id,day,samples,active_seconds,idle_seconds,keys_count,mouse_moves_count,clicks_count,available_samples,unavailable_samples)
        SELECT device_id, (timestamp AT TIME ZONE 'UTC')::date, count(*),
               sum(active_seconds), sum(idle_seconds), sum(keys_count),
               sum(mouse_moves_count), sum(clicks_count),
               count(*) FILTER (WHERE input_available IS TRUE),count(*) FILTER (WHERE input_available IS FALSE)
        FROM moved WHERE (timestamp AT TIME ZONE 'UTC')::date >= summary_cutoff
        GROUP BY device_id, (timestamp AT TIME ZONE 'UTC')::date
        ON CONFLICT (day, device_id) DO UPDATE SET
            samples = d.samples + EXCLUDED.samples,
            active_seconds = d.active_seconds + EXCLUDED.active_seconds,
            idle_seconds = d.idle_seconds + EXCLUDED.idle_seconds,
            keys_count = d.keys_count + EXCLUDED.keys_count,
            mouse_moves_count = d.mouse_moves_count + EXCLUDED.mouse_moves_count,
            clicks_count = d.clicks_count + EXCLUDED.clicks_count,
            available_samples = d.available_samples + EXCLUDED.available_samples,
            unavailable_samples = d.unavailable_samples + EXCLUDED.unavailable_samples
        RETURNING 1
    ) SELECT count(*) INTO input_count FROM moved;
    DELETE FROM public.resource_metrics_daily WHERE day < summary_cutoff;
    DELETE FROM public.input_metrics_daily WHERE day < summary_cutoff;
    INSERT INTO public.maintenance_status VALUES ('metrics-retention', now(), resource_count, input_count)
    ON CONFLICT (job) DO UPDATE SET last_success_at = EXCLUDED.last_success_at,
        resource_rows_compacted = EXCLUDED.resource_rows_compacted,
        input_rows_compacted = EXCLUDED.input_rows_compacted;
    RAISE NOTICE 'Compacted resources=%, input=%', resource_count, input_count;
END $$;
\else
\echo 'Maintenance already running; skipping this batch.'
\endif
COMMIT;
