#!/bin/sh
# Install an already built artifact; no downloads, compilation or config overwrite.
set -eu
[ "$(id -u)" -eq 0 ] || { echo 'Run as root' >&2; exit 1; }
[ "$#" -eq 2 ] || { echo 'Usage: install-service.sh BINARY CONFIG_ENV' >&2; exit 2; }
[ -f "$1" ] && [ -f "$2" ] || exit 2
exec 9>/run/lock/activity-monitor-lifecycle.lock
flock -n 9 || { echo "Another lifecycle operation is running" >&2; exit 1; }
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Existing deployments need an explicit migration of their identities and ownership.
if systemctl cat activity-monitor-agent.service >/dev/null 2>&1; then
    echo 'Existing service detected: use a reviewed migration/update procedure' >&2
    exit 2
fi
if [ -e /var/lib/activity-monitor/device_id.json ]; then
    echo 'Existing identity detected: migrate ownership and cache explicitly' >&2
    exit 2
fi
getent group activity-monitor >/dev/null || groupadd --system activity-monitor
id activity-monitor >/dev/null 2>&1 || useradd --system --gid activity-monitor --home-dir /var/lib/activity-monitor --shell /usr/sbin/nologin activity-monitor
install -d -m 0755 /opt/activity-monitor/bin
install -d -m 0750 -o root -g activity-monitor /etc/activity-monitor
install -d -m 0700 -o activity-monitor -g activity-monitor /var/lib/activity-monitor /var/log/activity-monitor
if [ ! -e /etc/activity-monitor/.env ]; then
    install -m 0640 -o root -g activity-monitor "$2" /etc/activity-monitor/.env
fi
install -m 0755 "$1" /opt/activity-monitor/bin/activity-monitor-agent
install -m 0644 "$script_dir/activity-monitor-agent.service" /etc/systemd/system/activity-monitor-agent.service
systemctl daemon-reload
systemctl enable activity-monitor-agent.service
echo 'Installed and enabled for boot. Start explicitly after configuration review.'
