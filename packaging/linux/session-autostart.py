#!/usr/bin/env python3
"""Enable or remove autostart for the one enrolled user; preserve credentials/outbox."""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import pwd
import subprocess

spec = importlib.util.spec_from_file_location('manager', Path(__file__).with_name('manage-session.py'))
manager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(manager)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('enable','disable'))
    args = parser.parse_args()
    if os.geteuid() != 0: parser.error('Run as root')
    lock = os.open('/run/lock/activity-monitor-lifecycle.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    base = manager.BASE
    manager.enrollment.root_directory(base)
    if (base / 'session-operation.json').exists(): raise RuntimeError('Repair the pending operation first')
    record = json.loads(manager.read_file(base / 'session-enrollment.json', 0))
    user = pwd.getpwuid(record['uid'])
    if user.pw_uid == 0 or user.pw_name != record.get('user',user.pw_name): raise RuntimeError('Enrollment principal mismatch')
    directory = base / 'sessions' / str(user.pw_uid)
    manager.enrollment.root_directory(base / 'sessions')
    manager.enrollment.root_directory(directory)
    flag = directory / 'autostart.enabled'
    unit_name = 'activity-monitor-session.service'
    unit = Path('/etc/systemd/user') / unit_name
    content = Path(__file__).with_name(unit_name).read_text(encoding='utf-8')
    if args.action == 'enable':
        if record.get('state') != 'active': raise RuntimeError('Activate the enrollment before enabling autostart')
        manager.read_file(directory / 'session.env', user.pw_uid)
        if unit.exists() or unit.is_symlink():
            if manager.read_file(unit,0) != content: raise RuntimeError('Existing user unit differs; explicit package upgrade required')
        else:
            manager.enrollment.root_directory(unit.parent,0o755)
            manager.enrollment.atomic_write(unit,content,0,0,0o644)
        subprocess.run(['systemctl','--global','enable',unit_name],check=True)
        manager.enrollment.atomic_write(flag,'enabled\n',0,0,0o644)
    else:
        if flag.is_symlink(): raise RuntimeError('Refusing symlink autostart marker')
        flag.unlink(missing_ok=True)
        fd=os.open(directory,os.O_RDONLY | os.O_DIRECTORY)
        try: os.fsync(fd)
        finally: os.close(fd)
    # If a user manager exists, reload it and stop only our unit on removal.
    runtime = Path('/run/user') / str(user.pw_uid)
    if (runtime / 'bus').exists():
        prefix = ['runuser','-u',user.pw_name,'--','env',f'XDG_RUNTIME_DIR={runtime}',f'DBUS_SESSION_BUS_ADDRESS=unix:path={runtime}/bus','systemctl','--user']
        subprocess.run(prefix+['daemon-reload'],check=True)
        if args.action == 'disable': subprocess.run(prefix+['stop',unit_name],check=True)
    print('Autostart enabled for next graphical login; no agent started.' if args.action == 'enable' else 'Autostart removed. Configuration, identity and outbox retained.')


if __name__ == '__main__': main()
