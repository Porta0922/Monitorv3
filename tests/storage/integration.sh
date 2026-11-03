#!/bin/sh
# Run only against an empty disposable PostgreSQL database.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
export MAINTENANCE_SQL_FILE="$ROOT/scripts/maintenance/retention.sql"
export MAINTENANCE_BATCH_SIZE=1

python3 - "$ROOT" <<'PY' | psql -X -v ON_ERROR_STOP=1
import pathlib, re, sys
source = (pathlib.Path(sys.argv[1]) / 'server/migrations/0001_server_schema.sql').read_text()
for table in ('devices', 'node_resource_metrics', 'input_activity_metrics'):
    ddl = re.search(r'CREATE TABLE IF NOT EXISTS ' + table + r' \([\s\S]*?\n            \)', source)
    assert ddl, table
    print(ddl.group(0) + ';')
PY
psql -X -v ON_ERROR_STOP=1 -f "$ROOT/server/migrations/0002_daily_metrics.sql"
psql -X -v ON_ERROR_STOP=1 -f "$ROOT/server/migrations/0010_input_quality.sql"

psql -X -v ON_ERROR_STOP=1 <<'SQL'
INSERT INTO devices(device_id, hostname, last_seen)
VALUES ('00000000-0000-0000-0000-000000000001', 'test', now());
INSERT INTO node_resource_metrics(device_id, timestamp, cpu_percent)
SELECT '00000000-0000-0000-0000-000000000001', now()-interval '20 days', n
FROM generate_series(20,80,60) n;
SQL
sh "$ROOT/scripts/db-maintenance.sh" --once
sh "$ROOT/scripts/db-maintenance.sh" --once
sh "$ROOT/scripts/db-maintenance.sh" --once
psql -X -v ON_ERROR_STOP=1 <<'SQL'
DO $$ BEGIN
  IF (SELECT samples FROM resource_metrics_daily) <> 2 OR
     (SELECT cpu_sum FROM resource_metrics_daily) <> 100 OR
     EXISTS (SELECT 1 FROM node_resource_metrics) THEN
    RAISE EXCEPTION 'Compaction/retry failed';
  END IF;
END $$;
INSERT INTO node_resource_metrics(device_id, timestamp, cpu_percent)
VALUES ('00000000-0000-0000-0000-000000000001', now()-interval '20 days', 50);
ALTER TABLE resource_metrics_daily ADD CONSTRAINT injected_failure CHECK (samples <= 2);
SQL
if sh "$ROOT/scripts/db-maintenance.sh" --once; then
    echo 'Expected failure from injected constraint' >&2
    exit 1
fi
psql -X -v ON_ERROR_STOP=1 <<'SQL'
DO $$ BEGIN
  IF (SELECT count(*) FROM node_resource_metrics) <> 1 THEN
    RAISE EXCEPTION 'Rollback lost data';
  END IF;
END $$;
SQL
echo 'PostgreSQL/psql integration passed'
