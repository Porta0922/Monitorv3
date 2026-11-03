#!/usr/bin/env python3
"""Repair/reinstall the managed service from a trusted local artifact; preserve all data.
Does not change legacy/custom data paths or start the service.
"""
import argparse
import fcntl
import grp
import hashlib
import os
from pathlib import Path
import pwd
import stat
import subprocess
import tempfile


def secure_path(path, owner=0):
    for part in (path, *path.parents):
        info = part.lstat()
        if stat.S_ISLNK(info.st_mode) or info.st_uid != (owner if part == path else 0) or info.st_mode & 0o022:
            raise RuntimeError(f'Untrusted ownership, permissions or symlink: {part}')


def replace(path, content, mode):
    fd, name = tempfile.mkstemp(prefix='.repair-',dir=path.parent)
    try:
        with os.fdopen(fd,'wb') as stream:
            stream.write(content); stream.flush(); os.fchmod(stream.fileno(),mode); os.fsync(stream.fileno())
        os.replace(name,path)
        directory=os.open(path.parent,os.O_RDONLY | os.O_DIRECTORY)
        try: os.fsync(directory)
        finally: os.close(directory)
    finally:
        if os.path.exists(name): os.unlink(name)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--sha256',required=True)
    args=parser.parse_args()
    if os.geteuid()!=0: parser.error('Run as root')
    lock=os.open('/run/lock/activity-monitor-lifecycle.lock',os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW,0o600)
    fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
    base=Path('/etc/activity-monitor')
    secure_path(base); secure_path(base / '.env')
    if (base / 'session-operation.json').exists(): raise RuntimeError('Repair the pending session operation first')
    account=pwd.getpwnam('activity-monitor'); group=grp.getgrnam('activity-monitor')
    for path in (Path('/var/lib/activity-monitor'),Path('/var/log/activity-monitor')):
        secure_path(path,account.pw_uid)
    unit=Path('/etc/systemd/system/activity-monitor-agent.service')
    template=Path(__file__).with_name(unit.name).read_bytes()
    if unit.exists() or unit.is_symlink():
        secure_path(unit)
        if unit.read_bytes()!=template: raise RuntimeError('Custom/legacy unit requires explicit migration')
    status=subprocess.run(['systemctl','show',unit.name,'--property=ActiveState','--property=LoadState'],capture_output=True,text=True)
    properties=dict(line.split('=',1) for line in status.stdout.splitlines() if '=' in line)
    if status.returncode and properties.get('LoadState')!='not-found': raise RuntimeError('Cannot determine service state')
    if properties.get('ActiveState') not in ('inactive','failed'): raise RuntimeError('Stop the system service before repair')
    binary=Path('/opt/activity-monitor/bin/activity-monitor-agent')
    secure_path(binary.parent)
    if binary.exists() or binary.is_symlink(): secure_path(binary)
    # Read once: the exact bytes hashed are the bytes installed.
    candidate=args.binary.read_bytes()
    if hashlib.sha256(candidate).hexdigest()!=args.sha256.lower(): raise RuntimeError('SHA256 mismatch')
    if not candidate.startswith(b'\x7fELF'): raise RuntimeError('Expected an ELF artifact for Linux')
    # Repair is repeatable if interrupted between these atomic replacements.
    replace(binary,candidate,0o755)
    replace(unit,template,0o644)
    os.chown(base / '.env',0,group.gr_gid); os.chmod(base / '.env',0o640)
    subprocess.run(['systemctl','daemon-reload'],check=True)
    subprocess.run(['systemctl','enable',unit.name],check=True)
    print('Managed service repaired and enabled for boot; left stopped. Identity, configuration and queues retained.')


if __name__=='__main__': main()
