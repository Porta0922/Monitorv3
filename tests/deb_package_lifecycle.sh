#!/bin/sh
# Real dpkg/dependency/binary checks in a disposable container.
# systemd is simulated: this does NOT certify service recovery or a desktop.
set -eu
[ -f /.dockerenv ] || { echo 'Disposable container required' >&2; exit 2; }
[ -f /package.deb ] || exit 2
export DEBIAN_FRONTEND=noninteractive
mkdir -p /usr/local/bin
cat > /usr/local/bin/systemctl <<'SH'
#!/bin/sh
case "$1" in
is-active) exit 3;;
daemon-reload|enable|disable|stop) exit 0;;
*) echo 'Unexpected service operation in package fixture' >&2; exit 2;;
esac
SH
chmod 755 /usr/local/bin/systemctl
printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d
chmod 755 /usr/sbin/policy-rc.d
apt-get update -qq
# Resolve the real package dependencies first. Only afterwards replace systemctl
# inside this disposable container, so installing systemd cannot overwrite the
# simulation and dpkg's restricted PATH still resolves it.
apt-get satisfy -y -qq --no-install-recommends "$(dpkg-deb -f /package.deb Depends)" > /tmp/dependencies.log 2>&1 || { tail -40 /tmp/dependencies.log; exit 1; }
cp /usr/local/bin/systemctl /bin/systemctl
cp /usr/local/bin/systemctl /usr/bin/systemctl
dpkg -i /package.deb > /tmp/install.log 2>&1 || { tail -40 /tmp/install.log; exit 1; }
test "$(dpkg-query -W -f='${Version}' activity-monitor-agent)" = "$(dpkg-deb -f /package.deb Version)"
/opt/activity-monitor/bin/activity-monitor-agent --version-json
test "$(stat -c %a /etc/activity-monitor/.env)" = 640
test "$(stat -c %U /etc/activity-monitor/.env)" = root
printf '\n# Synthetic lifecycle sentinel\n' >> /etc/activity-monitor/.env
cp /etc/activity-monitor/.env /tmp/config-before
printf 'synthetic-identity\n' > /var/lib/activity-monitor/device_id.json
printf 'synthetic-pending-queue\n' > /var/lib/activity-monitor/agent_service_cache.db
sha256sum /var/lib/activity-monitor/device_id.json /var/lib/activity-monitor/agent_service_cache.db > /tmp/data-before
dpkg -i /package.deb > /tmp/reinstall.log 2>&1 || { tail -40 /tmp/reinstall.log; exit 1; }
cmp /tmp/config-before /etc/activity-monitor/.env
sha256sum -c /tmp/data-before
dpkg --remove activity-monitor-agent
cmp /tmp/config-before /etc/activity-monitor/.env
sha256sum -c /tmp/data-before
dpkg --purge activity-monitor-agent
cmp /tmp/config-before /etc/activity-monitor/.env
sha256sum -c /tmp/data-before
test ! -e /opt/activity-monitor/bin/activity-monitor-agent
echo 'PASS: real DEB dependencies, binary version, reinstall/remove/purge preserve configuration and data; service manager simulated'
