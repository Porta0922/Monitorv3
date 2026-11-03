"""Bounded production role checks in a disposable DB; never uses pilot containers."""
import secrets, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
NAME = "monitor-role-check-" + secrets.token_hex(4)

def docker(*args, sql=None):
    result = subprocess.run(["docker", *args], input=sql, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr[-2500:])
    return result.stdout

def psql(sql, *variables):
    return docker("exec", "-i", NAME, "psql", "-X", "-U", "postgres", "-d", "activity_monitor", "-v", "ON_ERROR_STOP=1", *variables, sql=sql)

try:
    docker("run", "--rm", "-d", "--name", NAME, "--network", "none", "-e", "POSTGRES_PASSWORD=" + secrets.token_hex(32), "-e", "POSTGRES_DB=activity_monitor", "timescale/timescaledb:latest-pg15@sha256:6343bdc87ca132c6b53acb26113a6bad1821d188fa39975a745f32ddd9757634")
    for attempt in range(40):
        try:
            docker("exec", NAME, "pg_isready", "-h", "127.0.0.1", "-U", "postgres", "-d", "activity_monitor")
            break
        except RuntimeError:
            time.sleep(1)
    else:
        raise RuntimeError("Disposable database did not start")
    psql("CREATE ROLE monitor_owner LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE; CREATE ROLE monitor_app LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE; CREATE ROLE monitor_maintenance LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE; ALTER DATABASE activity_monitor OWNER TO monitor_owner; ALTER SCHEMA public OWNER TO monitor_owner;")
    for migration in sorted((ROOT / "server/migrations").glob("*.sql")):
        psql("SET ROLE monitor_owner; BEGIN;\n" + migration.read_text() + "\nCOMMIT;")
    psql("SET ROLE monitor_owner; CREATE TABLE _sqlx_migrations(version bigint PRIMARY KEY, success boolean, checksum bytea);")
    psql("SET ROLE monitor_owner;" + (ROOT / "scripts/production/grants.sql").read_text())
    psql("""DO $$ BEGIN
      IF has_schema_privilege('monitor_app','public','CREATE') OR has_schema_privilege('monitor_maintenance','public','CREATE') THEN RAISE EXCEPTION 'Unexpected DDL privilege'; END IF;
      IF has_table_privilege('monitor_app','_sqlx_migrations','UPDATE') OR has_table_privilege('monitor_maintenance','devices','DELETE') THEN RAISE EXCEPTION 'Unexpected write privilege'; END IF;
      IF NOT has_table_privilege('monitor_app','devices','INSERT') OR NOT has_table_privilege('monitor_app','desktop_activity','SELECT') THEN RAISE EXCEPTION 'Missing application privilege'; END IF;
      IF EXISTS(SELECT FROM pg_roles WHERE rolname IN ('monitor_app','monitor_maintenance') AND (rolsuper OR rolcreatedb OR rolcreaterole)) THEN RAISE EXCEPTION 'Elevated service role'; END IF;
    END $$;""")
    psql("SET ROLE monitor_maintenance;\n" + (ROOT / "scripts/maintenance/retention.sql").read_text(), "-v", "raw_days=14", "-v", "summary_days=365", "-v", "batch_size=10")
    for job in ['deduplication','security','inventory-changes','activity','sessions','usb','wifi','audit','resolved-alerts']:
        psql("SET ROLE monitor_maintenance;\n" + (ROOT / "scripts/maintenance/history-retention.sql").read_text(), "-v", "job=" + job, "-v", "retention_days=180", "-v", "batch_size=10")
    print("PASS: all migrations as owner, role boundaries, metrics and nine history jobs on empty isolated DB")
finally:
    subprocess.run(["docker", "rm", "-f", NAME], capture_output=True)
