-- Summary storage is available before the API starts.
CREATE TABLE IF NOT EXISTS public.resource_metrics_daily (
    device_id uuid NOT NULL REFERENCES public.devices(device_id) ON DELETE CASCADE,
    day date NOT NULL,
    samples bigint NOT NULL,
    cpu_sum double precision NOT NULL,
    cpu_max double precision NOT NULL,
    memory_percent_sum double precision NOT NULL,
    memory_percent_max double precision NOT NULL,
    memory_used_mb_sum double precision NOT NULL,
    PRIMARY KEY (day, device_id)
);
CREATE TABLE IF NOT EXISTS public.input_metrics_daily (
    device_id uuid NOT NULL REFERENCES public.devices(device_id) ON DELETE CASCADE,
    day date NOT NULL,
    samples bigint NOT NULL,
    active_seconds bigint NOT NULL,
    idle_seconds bigint NOT NULL,
    keys_count bigint NOT NULL,
    mouse_moves_count bigint NOT NULL,
    clicks_count bigint NOT NULL,
    PRIMARY KEY (day, device_id)
);
CREATE TABLE IF NOT EXISTS public.maintenance_status (
    job text PRIMARY KEY,
    last_success_at timestamptz NOT NULL,
    resource_rows_compacted bigint NOT NULL,
    input_rows_compacted bigint NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_resource_metrics_daily_device_day ON public.resource_metrics_daily(device_id, day);
CREATE INDEX IF NOT EXISTS idx_input_metrics_daily_device_day ON public.input_metrics_daily(device_id, day);
