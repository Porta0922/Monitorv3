#!/bin/sh
# Periodic bounded compaction. Use --once for a single batch.
set -eu
if [ -n "${PGPASSWORD_FILE:-}" ]; then
    [ -z "${PGPASSWORD:-}" ] || { echo 'Use PGPASSWORD or PGPASSWORD_FILE, not both' >&2; exit 2; }
    PGPASSWORD=$(cat "$PGPASSWORD_FILE")
    [ -n "$PGPASSWORD" ] || { echo 'Empty database password file' >&2; exit 2; }
    export PGPASSWORD
fi
export PGHOST="${PGHOST:-postgres}"
export PGPORT="${PGPORT:-5432}"
export PGUSER="${PGUSER:-monitor_user}"
export PGDATABASE="${PGDATABASE:-activity_monitor}"
export PGCONNECT_TIMEOUT=10
RAW_DAYS="${METRICS_RAW_RETENTION_DAYS:-14}"
SUMMARY_DAYS="${METRICS_SUMMARY_RETENTION_DAYS:-365}"
BATCH_SIZE="${MAINTENANCE_BATCH_SIZE:-5000}"
INTERVAL="${MAINTENANCE_INTERVAL_SECONDS:-300}"
HISTORY_ENABLED="${HISTORY_RETENTION_ENABLED:-false}"
HISTORY_SQL_FILE="${HISTORY_MAINTENANCE_SQL_FILE:-/maintenance/history-retention.sql}"
case "$HISTORY_ENABLED" in true|false) ;; *) echo "HISTORY_RETENTION_ENABLED must be true or false" >&2; exit 2;; esac
SQL_FILE="${MAINTENANCE_SQL_FILE:-/maintenance/retention.sql}"
for value in "$RAW_DAYS" "$SUMMARY_DAYS" "$BATCH_SIZE" "$INTERVAL"; do
    case "$value" in ''|*[!0-9]*) echo 'Maintenance settings must be positive integers' >&2; exit 2;; esac
    [ "$value" -gt 0 ] || exit 2
done
[ "$SUMMARY_DAYS" -gt "$RAW_DAYS" ] && [ "$SUMMARY_DAYS" -le 3650 ] || exit 2
[ "$BATCH_SIZE" -le 100000 ] && [ "$INTERVAL" -le 86400 ] || exit 2
case "${1:-}" in ''|--once) ;; *) echo 'Usage: db-maintenance.sh [--once]' >&2; exit 2;; esac
run_history() {
    job="$1"
    days="$2"
    case "$days" in ''|*[!0-9]*) echo "Invalid retention for $job" >&2; return 2;; esac
    [ "$days" -le 3650 ] || return 2
    [ "$days" -ne 0 ] || return 0
    psql -X --no-password -v ON_ERROR_STOP=1 -v job="$job" \
        -v retention_days="$days" -v batch_size="$BATCH_SIZE" -f "$HISTORY_SQL_FILE"
}
trap 'exit 0' TERM INT
while :; do
    result=0
    echo "[$(date -u '+%Y-%m-%dT%H:%M:%SZ')] Starting metrics maintenance"
    if psql -X --no-password -v ON_ERROR_STOP=1 \
        -v raw_days="$RAW_DAYS" -v summary_days="$SUMMARY_DAYS" \
        -v batch_size="$BATCH_SIZE" -f "$SQL_FILE"; then
        echo 'Metrics maintenance succeeded'
    else
        echo 'Metrics maintenance failed; transaction rolled back' >&2
        result=1
    fi
    if [ "$HISTORY_ENABLED" = true ]; then
        run_history deduplication "${DEDUPE_RETENTION_DAYS:-32}" || result=1
        run_history security "${SECURITY_EVENT_RETENTION_DAYS:-0}" || result=1
        run_history inventory-changes "${INVENTORY_CHANGE_RETENTION_DAYS:-90}" || result=1
        run_history activity "${ACTIVITY_RETENTION_DAYS:-30}" || result=1
        run_history sessions "${SESSION_EVENT_RETENTION_DAYS:-30}" || result=1
        run_history usb "${USB_RETENTION_DAYS:-90}" || result=1
        run_history wifi "${WIFI_RETENTION_DAYS:-90}" || result=1
        run_history audit "${AUDIT_RETENTION_DAYS:-180}" || result=1
        run_history resolved-alerts "${RESOLVED_ALERT_RETENTION_DAYS:-180}" || result=1
    fi
    [ "${1:-}" != '--once' ] || exit "$result"
    sleep "$INTERVAL" &
    wait $! || true
done
