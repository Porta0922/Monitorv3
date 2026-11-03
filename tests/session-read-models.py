"""Bounded production role checks in a disposable DB; never uses pilot containers."""
import secrets, subprocess, time
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
NAME = "monitor-functional-check-" + secrets.token_hex(4)

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
    psql((ROOT/'tests/session-read-models.sql').read_text())
    query=(ROOT/'server/src/domains/device/session_activity.sql').read_text().strip().rstrip(';')
    result=psql("PREPARE session_read(uuid,date,bigint,bigint) AS "+query+"; EXECUTE session_read('00000000-0000-0000-0000-000000000001',CURRENT_DATE,0,1);", "-tA")
    import json
    data=json.loads(next(line for line in result.splitlines() if line.startswith('{')))
    assert len(data['items'])==1 and data['total']==2
    assert data['totals']['active_person_seconds']==14 and data['totals']['idle_person_seconds']==26
    assert data['totals']['keys_count']==7 and data['totals']['clicks_count']==3 and data['totals']['mouse_moves_count']==9
    assert data['totals']['available_samples']==2 and data['totals']['unavailable_samples']==1
    empty=psql("PREPARE session_read(uuid,date,bigint,bigint) AS "+query+"; EXECUTE session_read('00000000-0000-0000-0000-000000000001',CURRENT_DATE+1,0,50);", "-tA")
    no_data=json.loads(next(line for line in empty.splitlines() if line.startswith('{')))
    assert no_data['totals']['keys_count'] is None and no_data['totals']['active_person_seconds'] is None and no_data['items']==[]
    psql("UPDATE session_events SET timestamp=timestamp-interval '40 days'")
    aggregate="SELECT COALESCE(jsonb_agg((to_jsonb(v)-'detail_samples'-'summary_samples') ORDER BY v.day,v.principal,v.session_id),'[]'::jsonb) FROM session_activity_combined v"
    before=psql(aggregate,"-tA").strip()
    for _ in range(8):
        psql((ROOT/'scripts/maintenance/history-retention.sql').read_text(),"-v","job=sessions","-v","retention_days=30","-v","batch_size=2")
    after=psql(aggregate,"-tA").strip()
    assert json.loads(before)==json.loads(after),'Compaction changed session counters'
    assert psql("SELECT count(*) FROM session_events","-tA").strip()=='0'
    print('PASS: transactional bounded session compaction preserves full aggregates across retries')
    print('PASS: all migrations; independent desktops, stale reports, empty/out-of-order snapshots, full-day paginated totals and unavailable/null counters')
finally:
    subprocess.run(['docker','rm','-f',NAME],capture_output=True)
