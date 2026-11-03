#!/bin/sh
# Remove only the system service and its executable. Preserve identity/config/queue/logs.
set -eu
[ "$(id -u)" -eq 0 ] || { echo 'Run as root' >&2; exit 1; }
[ "$#" -eq 0 ] || { echo 'Usage: uninstall-service.sh (data is always retained)' >&2; exit 2; }
exec 9>/run/lock/activity-monitor-lifecycle.lock
flock -n 9 || { echo 'Another lifecycle operation is running' >&2; exit 1; }
unit=activity-monitor-agent.service
systemctl stop "$unit"
systemctl disable "$unit"
rm -f /etc/systemd/system/activity-monitor-agent.service
systemctl daemon-reload
rm -f /opt/activity-monitor/bin/activity-monitor-agent
echo 'System service removed; configuration, identity, queue, logs and service account retained.'
echo 'Session services are separate and must be removed in each enrolled user session.'
