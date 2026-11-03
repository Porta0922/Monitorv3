#!/bin/sh
# Run ONLY inside a disposable root container. systemctl is simulated, filesystem is real.
set -eu
[ -f /.dockerenv ] || { echo 'Disposable Docker container required' >&2; exit 2; }
mkdir -p /tmp/lifecycle-bin
cat > /tmp/lifecycle-bin/systemctl <<'MOCK'
#!/bin/sh
case "$1" in
cat) test -f /etc/systemd/system/activity-monitor-agent.service;;
is-active) test "$(cat /tmp/lifecycle-state 2>/dev/null)" = active;;
stop) echo inactive > /tmp/lifecycle-state;;
start) if grep -q broken /opt/activity-monitor/bin/activity-monitor-agent; then exit 1; fi; echo active > /tmp/lifecycle-state;;
enable|disable|daemon-reload) exit 0;;
*) exit 2;;
esac
MOCK
chmod +x /tmp/lifecycle-bin/systemctl
export PATH=/tmp/lifecycle-bin:$PATH
printf 'original\n' > /tmp/original
printf 'candidate\n' > /tmp/candidate
printf 'broken\n' > /tmp/broken
printf 'AGENT_MODE=service\n' > /tmp/config
sh /repo/packaging/linux/install-service.sh /tmp/original /tmp/config
test "$(stat -c %a /etc/activity-monitor/.env)" = 640
printf 'identity' > /var/lib/activity-monitor/device_id.json
printf 'pending' > /var/lib/activity-monitor/agent_service_cache.db
systemctl start activity-monitor-agent.service
hash=$(sha256sum /tmp/candidate | cut -d ' ' -f 1)
if sh /repo/packaging/linux/update-service.sh /tmp/candidate "$(printf '0%.0s' $(seq 1 64))"; then exit 1; fi
cmp /tmp/original /opt/activity-monitor/bin/activity-monitor-agent
systemctl is-active --quiet activity-monitor-agent.service
sh /repo/packaging/linux/update-service.sh /tmp/candidate "$hash"
cmp /tmp/candidate /opt/activity-monitor/bin/activity-monitor-agent
systemctl is-active --quiet activity-monitor-agent.service
bad_hash=$(sha256sum /tmp/broken | cut -d ' ' -f 1)
if sh /repo/packaging/linux/update-service.sh /tmp/broken "$bad_hash"; then exit 1; fi
cmp /tmp/candidate /opt/activity-monitor/bin/activity-monitor-agent
systemctl is-active --quiet activity-monitor-agent.service
systemctl stop activity-monitor-agent.service
sh /repo/packaging/linux/update-service.sh /tmp/original "$(sha256sum /tmp/original | cut -d ' ' -f 1)"
if systemctl is-active --quiet activity-monitor-agent.service; then exit 1; fi
sh /repo/packaging/linux/uninstall-service.sh
test ! -e /etc/systemd/system/activity-monitor-agent.service
test ! -e /opt/activity-monitor/bin/activity-monitor-agent
test "$(cat /var/lib/activity-monitor/device_id.json)" = identity
test "$(cat /var/lib/activity-monitor/agent_service_cache.db)" = pending
cmp /tmp/config /etc/activity-monitor/.env
echo 'PASS: install, hash rejection, update, rollback, stopped-state preservation, uninstall preserving data (systemctl simulated)'
