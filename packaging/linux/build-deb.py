#!/usr/bin/env python3
"""Package a prebuilt Linux artifact. Requires dpkg-deb; never compiles/downloads."""
import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary',required=True,type=Path)
parser.add_argument('--sha256',required=True)
parser.add_argument('--version',required=True)
parser.add_argument('--architecture',choices=('amd64','arm64'),required=True)
parser.add_argument('--output',required=True,type=Path)
args=parser.parse_args()
if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.]+)?',args.version): parser.error('Invalid Debian package version')
artifact=args.binary.read_bytes()
if not artifact.startswith(b'\x7fELF') or hashlib.sha256(artifact).hexdigest()!=args.sha256.lower(): parser.error('Artifact/hash mismatch')
root=Path(__file__).resolve().parents[2]
args.output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='monitor-deb-') as temporary:
    stage=Path(temporary)
    def write(relative,content,mode=0o644):
        path=stage/relative;path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(content if isinstance(content,bytes) else content.encode());path.chmod(mode)
    write('opt/activity-monitor/bin/activity-monitor-agent',artifact,0o755)
    for source in (root/'packaging/linux').iterdir():
        if source.suffix in ('.py','.sh','.service') and source.name!='build-deb.py':
            write('opt/activity-monitor/maintenance/'+source.name,source.read_bytes(),0o755 if source.suffix in ('.py','.sh') else 0o644)
    write('opt/activity-monitor/maintenance/data-lifecycle.py',(root/'packaging/common/data-lifecycle.py').read_bytes(),0o755)
    write('etc/xdg/autostart/activity-monitor-session.desktop',(root/'packaging/linux/activity-monitor-session.desktop').read_bytes())
    write('usr/share/activity-monitor/agent.env.example',(root/'packaging/agent.env.example').read_bytes())
    write('lib/systemd/system/activity-monitor-agent.service',(root/'packaging/linux/activity-monitor-agent.service').read_bytes())
    write('usr/share/applications/activity-monitor.desktop',(root/'packaging/linux/activity-monitor.desktop').read_bytes())
    write('DEBIAN/control',f"Package: activity-monitor-agent\nVersion: {args.version}\nArchitecture: {args.architecture}\nMaintainer: ActivityMonitor administrators\nDepends: libc6, libssl3, libx11-6, libxi6, libxtst6, libxcb1, libxrandr2, libdbus-1-3, libpipewire-0.3-0, libgbm1, libwayland-client0, libxkbcommon0, libegl1, python3, systemd, util-linux, openssl, xdg-utils, x11-utils\nDescription: ActivityMonitor system agent and optional desktop companion\n Native monitoring agent with durable offline queue.\n")
    write('DEBIAN/postinst',"""#!/bin/sh
set -eu
case "${1:-}" in configure|abort-upgrade|abort-remove|abort-deconfigure) ;; *) exit 0;; esac
getent group activity-monitor >/dev/null || groupadd --system activity-monitor
id activity-monitor >/dev/null 2>&1 || useradd --system --gid activity-monitor --home-dir /var/lib/activity-monitor --shell /usr/sbin/nologin activity-monitor
for path in /etc/activity-monitor /var/lib/activity-monitor /var/log/activity-monitor; do
    [ ! -L "$path" ] || { echo "Refusing symlink: $path" >&2; exit 1; }
done
install -d -m 0750 -o root -g activity-monitor /etc/activity-monitor
install -d -m 0700 -o activity-monitor -g activity-monitor /var/lib/activity-monitor /var/log/activity-monitor
# Config is administrator data, not a dpkg conffile: purge must not lose the outbox key.
if [ ! -e /etc/activity-monitor/.env ]; then
    install -m 0640 -o root -g activity-monitor /usr/share/activity-monitor/agent.env.example /etc/activity-monitor/.env
fi
systemctl daemon-reload
systemctl enable activity-monitor-agent.service
python3 /opt/activity-monitor/maintenance/package-sessions.py restore
echo 'Installed; service left stopped. Configure credentials before starting.'
""",0o755)
    write('DEBIAN/prerm',"""#!/bin/sh
set -eu
case "${1:-}" in remove|upgrade|deconfigure)
    systemctl stop activity-monitor-agent.service
    python3 /opt/activity-monitor/maintenance/package-sessions.py prepare

;; esac
""",0o755)
    write('DEBIAN/postrm',"""#!/bin/sh
set -eu
case "${1:-}" in remove|purge)
    systemctl disable activity-monitor-agent.service || true
    # Successful removal retains data and credentials, but not the pending operation marker.
    if [ ! -L /etc/activity-monitor/package-session-operation.json ]; then
        rm -f -- /etc/activity-monitor/package-session-operation.json
    fi
;; esac
systemctl daemon-reload
# Explicitly retain account, configuration, identity, session secrets and pending queues.
""",0o755)
    subprocess.run(['dpkg-deb','--root-owner-group','--build',str(stage),str(args.output.resolve())],check=True)
print(args.output.resolve())
