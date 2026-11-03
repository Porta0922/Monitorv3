-- NULL preserves unknown quality in legacy observations; never backfill as healthy.
ALTER TABLE input_activity_metrics ADD COLUMN input_available boolean;
ALTER TABLE input_metrics_daily ADD COLUMN available_samples bigint NOT NULL DEFAULT 0;
ALTER TABLE input_metrics_daily ADD COLUMN unavailable_samples bigint NOT NULL DEFAULT 0;
