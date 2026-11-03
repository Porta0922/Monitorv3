#!/usr/bin/env python3
"""Recoverable session autostart boundary for dpkg operations. Keeps all queues and keys."""
import argparse, fcntl, importlib.util, json, os, select, signal, time
from pathlib import Path
spec=importlib.util.spec_from_file_location('manager',Path(__file__).with_name('manage-session.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=('prepare','restore','remove'));a=p.parse_args()
    if os.geteuid()!=0:p.error('Run as root')
    lock=os.open('/run/lock/activity-monitor-lifecycle.lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600);fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    base=m.BASE;m.enrollment.root_directory(base);journal=base/'package-session-operation.json'
    for name in ('session-operation.json','multi-session-operation.json'):
        if (base/name).exists():raise RuntimeError('Repair credential operation before changing packages')
    if a.action=='prepare':
        if journal.exists():raise RuntimeError('Pending package operation; restore or remove explicitly first')
        flags=[]
        root=base/'sessions'
        if root.exists():
            m.enrollment.root_directory(root)
            for directory in root.iterdir():
                if not directory.name.isdecimal():continue
                m.enrollment.root_directory(directory)
                for name in ('autostart.enabled','multi-autostart.enabled'):
                    path=directory/name
                    if path.exists():
                        m.read_file(path,0);flags.append(str(path.relative_to(base)))
        m.enrollment.atomic_write(journal,json.dumps({'version':1,'flags':flags})+'\n',0,0,0o600)
        for name in flags:(base/name).unlink()
        # Stop only processes executing this package's binary, using pidfds to avoid PID reuse.
        for entry in Path('/proc').iterdir():
            if not entry.name.isdecimal():continue
            try:
                if os.readlink(entry/'exe').removesuffix(' (deleted)')!='/opt/activity-monitor/bin/activity-monitor-agent':continue
                fd=os.pidfd_open(int(entry.name))
                try:
                    if os.readlink(entry/'exe').removesuffix(' (deleted)')!='/opt/activity-monitor/bin/activity-monitor-agent':continue
                    signal.pidfd_send_signal(fd,signal.SIGTERM)
                    if not select.select([fd],[],[],20)[0]:raise RuntimeError('Agent did not stop; package operation remains pending')
                finally:os.close(fd)
            except (FileNotFoundError,ProcessLookupError):continue
    else:
        if not journal.exists():return
        plan=json.loads(m.read_file(journal,0))
        if plan['version']!=1:raise RuntimeError('Unknown package journal')
        for name in plan['flags']:
            relative=Path(name)
            if len(relative.parts)!=3 or relative.parts[0]!='sessions' or not relative.parts[1].isdecimal() or relative.parts[2] not in ('autostart.enabled','multi-autostart.enabled'):raise RuntimeError('Invalid package journal path')
            m.enrollment.root_directory((base/relative).parent)
            if a.action=='restore':m.enrollment.atomic_write(base/relative,'enabled\n',0,0,0o644)
        journal.unlink()
    fd=os.open(base,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)
if __name__=='__main__':main()
