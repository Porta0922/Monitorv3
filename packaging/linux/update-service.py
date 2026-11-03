#!/usr/bin/env python3
"""Update a local artifact or recover an interrupted update by restoring the old binary."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

spec=importlib.util.spec_from_file_location('repair',Path(__file__).with_name('repair-service.py'))
repair=importlib.util.module_from_spec(spec);spec.loader.exec_module(repair)
BINARY=Path('/opt/activity-monitor/bin/activity-monitor-agent')
JOURNAL=BINARY.parent/'.update-journal.json'
UNIT='activity-monitor-agent.service'


def sync_remove(path):
    path.unlink()
    fd=os.open(path.parent,os.O_RDONLY | os.O_DIRECTORY)
    try: os.fsync(fd)
    finally: os.close(fd)


def restore(plan):
    name=plan.get('backup','')
    if not name.startswith('.update-') or not name.endswith('.previous') or Path(name).name!=name:
        raise RuntimeError('Invalid update backup name')
    backup=BINARY.parent/name
    repair.secure_path(backup)
    data=backup.read_bytes()
    if hashlib.sha256(data).hexdigest()!=plan['previous_sha256']: raise RuntimeError('Rollback artifact checksum mismatch')
    subprocess.run(['systemctl','stop',UNIT],check=True)
    repair.replace(BINARY,data,0o755)
    sync_remove(JOURNAL)
    print('Previous binary restored; service left stopped. Pending data retained.')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary',nargs='?',type=Path)
    parser.add_argument('sha256',nargs='?')
    parser.add_argument('--recover',action='store_true')
    args=parser.parse_args()
    if os.geteuid()!=0: parser.error('Run as root')
    lock=os.open('/run/lock/activity-monitor-lifecycle.lock',os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW,0o600)
    fcntl.flock(lock,fcntl.LOCK_EX | fcntl.LOCK_NB)
    repair.secure_path(BINARY.parent);repair.secure_path(BINARY)
    if JOURNAL.exists() or JOURNAL.is_symlink():
        repair.secure_path(JOURNAL)
        if not args.recover: raise RuntimeError('Pending update: run update-service.sh --recover')
        restore(json.loads(JOURNAL.read_text(encoding='utf-8')));return
    if args.recover: print('No interrupted update found.');return
    if not args.binary or not args.sha256: parser.error('BINARY and SHA256 required')
    data=args.binary.read_bytes()
    if hashlib.sha256(data).hexdigest()!=args.sha256.lower(): raise RuntimeError('SHA256 mismatch; service untouched')
    if not data.startswith(b'\x7fELF'): raise RuntimeError('Expected a Linux ELF artifact')
    previous=BINARY.read_bytes()
    name='.update-'+uuid.uuid4().hex+'.previous'
    repair.replace(BINARY.parent/name,previous,0o700)
    active=subprocess.run(['systemctl','is-active','--quiet',UNIT]).returncode==0
    plan={'version':1,'backup':name,'previous_sha256':hashlib.sha256(previous).hexdigest(),'candidate_sha256':args.sha256.lower(),'was_running':active}
    repair.replace(JOURNAL,(json.dumps(plan)+'\n').encode(),0o600)
    try:
        subprocess.run(['systemctl','stop',UNIT],check=True)
        repair.replace(BINARY,data,0o755)
        if active:
            subprocess.run(['systemctl','start',UNIT],check=True)
            time.sleep(3)
            subprocess.run(['systemctl','is-active','--quiet',UNIT],check=True)
        sync_remove(JOURNAL)
    except BaseException:
        # Preserve the journal and artifact if rollback fails; --recover is repeatable.
        restore(plan)
        if active: subprocess.run(['systemctl','start',UNIT],check=True)
        raise
    print('Update applied; identity/configuration/outbox retained. Rollback artifact retained for explicit housekeeping.')


if __name__=='__main__':main()
