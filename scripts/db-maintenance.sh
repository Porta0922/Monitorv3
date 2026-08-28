#!/bin/sh
# ActivityMonitor Enterprise - Database Maintenance Script
# Runs as a one-shot container via docker-compose or cron
#
# Schedule in docker-compose via:
#   db-maintenance:
#     image: timescale/timescaledb:latest-pg15
#     ...
#     command: ["-c", "0 3 * * 0 /maintenance.sh"]  # Every Sunday at 3 AM
#
# Or run manually:
#   docker compose run --rm db-maintenance
#
# Recommended schedule:
#   - Weekly (Sunday 3 AM): VACUUM, ANALYZE
#   - Monthly (1st of month): Chunk compression + reindex
#   - Quarterly: Drop old chunks (>12 months)

set -e

PGHOST="${PGHOST:-postgres}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-monitor_user}"
PGDATABASE="${PGDATABASE:-activity_monitor}"
PSQL="psql -h $PGHOST -p $PGPORT -U $PGUSER -d $PGDATABASE"

echo "============================================"
echo "  ActivityMonitor DB Maintenance"
echo "  $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo "============================================"

# Wait for PostgreSQL to be ready
echo "[*] Waiting for PostgreSQL..."
until $PSQL -c "SELECT 1" > /dev/null 2>&1; do
    sleep 2
done
echo "[+] PostgreSQL is ready"

# -------------------------------------------------------------------
# WEEKLY TASKS (run every Sunday)
# -------------------------------------------------------------------
echo ""
echo "--- WEEKLY: VACUUM and ANALYZE ---"

echo "[*] VACUUM ANALYZE activity_logs..."
$PSQL -c "VACUUM (VERBOSE) activity_logs;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE security_events..."
$PSQL -c "VACUUM (VERBOSE) security_events;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE security_alerts..."
$PSQL -c "VACUUM (VERBOSE) security_alerts;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE usb_events..."
$PSQL -c "VACUUM (VERBOSE) usb_events;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE wifi_events..."
$PSQL -c "VACUUM (VERBOSE) wifi_events;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE node_resource_metrics..."
$PSQL -c "VACUUM (VERBOSE) node_resource_metrics;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE input_summaries..."
$PSQL -c "VACUUM (VERBOSE) input_summaries;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE audit_events..."
$PSQL -c "VACUUM (VERBOSE) audit_events;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE devices..."
$PSQL -c "VACUUM (VERBOSE) devices;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE running_apps_current..."
$PSQL -c "VACUUM (VERBOSE) running_apps_current;" 2>&1 | tail -1 || true

echo "[*] VACUUM ANALYZE inventory..."
$PSQL -c "VACUUM (VERBOSE) inventory;" 2>&1 | tail -1 || true

echo "[+] Weekly VACUUM complete"

# -------------------------------------------------------------------
# MONTHLY TASKS (check if 1st of month or --monthly flag)
# -------------------------------------------------------------------
DAY_OF_MONTH=$(date -d)
RUN_MONTHLY="${RUN_MONTHLY:-false}"

if [ "$DAY_OF_MONTH" = "01" ] || [ "$RUN_MONTHLY" = "true" ]; then
    echo ""
    echo "--- MONTHLY: TimescaleDB chunk compression ---"

    # Compress chunks older than 7 days for high-volume tables
    COMPRESS_TABLES="activity_logs security_events usb_events wifi_events node_resource_metrics input_summaries"

    for TABLE in $COMPRESS_TABLES; do
        echo "[*] Compressing old chunks for $TABLE..."
        $PSQL -c "
            SELECT compress_chunk(ch)
            FROM show_chunks('$TABLE') AS ch
            WHERE ch < (NOW() - INTERVAL '7 days')
            AND NOT is_compressed(ch);
        " 2>&1 | grep -c "compress_chunk" || echo "    (no chunks to compress)"
    done

    echo "[+] Monthly compression complete"

    # Reindex bloated indexes
    echo ""
    echo "--- MONTHLY: REINDEX ---"
    for TABLE in $COMPRESS_TABLES; do
        echo "[*] Reindexing $TABLE..."
        $PSQL -c "REINDEX TABLE CONCURRENTLY $TABLE;" 2>&1 || true
    done
    echo "[+] Monthly REINDEX complete"
else
    echo ""
    echo "[*] Skipping monthly tasks (not 1st of month, use RUN_MONTHLY=true to force)"
fi

# -------------------------------------------------------------------
# QUARTERLY TASKS (check if quarter start or --quarterly flag)
# -------------------------------------------------------------------
MONTH_NUM=$(date +%m)
RUN_QUARTERLY="${RUN_QUARTERLY:-false}"

if { [ "$MONTH_NUM" = "01" ] || [ "$MONTH_NUM" = "04" ] || [ "$MONTH_NUM" = "07" ] || [ "$MONTH_NUM" = "10" ]; } && [ "$(date +%d)" = "01" ]; then
    RUN_QUARTERLY="true"
fi

if [ "$RUN_QUARTERLY" = "true" ]; then
    echo ""
    echo "--- QUARTERLY: Drop old chunks (>12 months) ---"

    DROP_TABLES="activity_logs security_events usb_events wifi_events node_resource_metrics input_summaries"

    for TABLE in $DROP_TABLES; do
        echo "[*] Dropping old chunks for $TABLE (>12 months)..."
        $PSQL -c "
            SELECT drop_chunk('$TABLE', older_than => INTERVAL '12 months');
        " 2>&1 || true
    done

    echo "[+] Quarterly chunk cleanup complete"
else
    echo ""
    echo "[*] Skipping quarterly tasks (use RUN_QUARTERLY=true to force)"
fi

# -------------------------------------------------------------------
# STATISTICS
# -------------------------------------------------------------------
echo ""
echo "--- Database Statistics ---"
$PSQL -c "
    SELECT
        relname AS table_name,
        pg_size_pretty(pg_total_relation_size(relid)) AS total_size,
        n_live_tup AS row_count
    FROM pg_stat_user_tables
    ORDER BY pg_total_relation_size(relid) DESC
    LIMIT 10;
" 2>&1 || true

echo ""
echo "============================================"
echo "  Maintenance complete: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
echo "============================================"
