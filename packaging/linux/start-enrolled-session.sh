#!/bin/sh
set -eu
uid=$(id -u)
base="/etc/activity-monitor/sessions/$uid"
[ -f "$base/multi-autostart.enabled" ] || exit 0
[ ! -e /etc/activity-monitor/multi-session-operation.json ] || exit 1
[ ! -e /etc/activity-monitor/package-session-operation.json ] || exit 1
export AGENT_MODE=session
export AGENT_CONFIG_FILE="$base/session.env"
attempt=0
while [ -f "$base/multi-autostart.enabled" ]; do
    code=0
    /opt/activity-monitor/bin/activity-monitor-agent || code=$?
    [ "$code" -ne 0 ] || exit 0
    attempt=$((attempt+1))
    [ "$attempt" -lt 10 ] || exit "$code"
    sleep 15
done
