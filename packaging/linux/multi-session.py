#!/usr/bin/env python3
"""Administer independent desktop principals while service and desktop agents are stopped."""
import argparse, fcntl, grp, importlib.util, json, os, pwd, secrets
from pathlib import Path
spec=importlib.util.spec_from_file_location('manager',Path(__file__).with_name('manage-session.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('action',choices=('enroll','activate','revoke','rotate','repair'))
    p.add_argument('--user');p.add_argument('--port',type=int);p.add_argument('--import-legacy',action='store_true')
    a=p.parse_args()
    if os.geteuid()!=0:p.error('Run as root')
    lock=os.open('/run/lock/activity-monitor-lifecycle.lock',os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
    fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    base=m.BASE;m.enrollment.root_directory(base)
    journal=base/'multi-session-operation.json'
    if (base/'session-operation.json').exists():raise RuntimeError('Repair legacy operation first')
    gid=grp.getgrnam('activity-monitor').gr_gid
    registry=base/'principals';m.enrollment.root_directory(registry,0o750);os.chown(registry,0,gid)
    if journal.exists():
        if a.action!='repair':raise RuntimeError('Use repair to finish pending multi-session operation')
        plan=json.loads(m.read_file(journal,0));user=pwd.getpwuid(plan['uid'])
        if plan['version']!=1 or user.pw_name!=plan['user']:raise RuntimeError('Journal principal mismatch')
    else:
        if not a.user:p.error('--user is required')
        user=pwd.getpwnam(a.user)
        if user.pw_uid==0 or user.pw_name=='activity-monitor':raise RuntimeError('Choose a desktop principal')
        m.stopped(user)
        directory=base/'sessions'/str(user.pw_uid)
        m.enrollment.root_directory(base/'sessions');m.enrollment.root_directory(directory)
        config=directory/'session.env';descriptor=registry/(str(user.pw_uid)+'.json')
        old=json.loads(m.read_file(descriptor,0)) if descriptor.exists() else None
        if a.action=='enroll' and old:raise RuntimeError('Principal already registered')
        if a.action!='enroll' and not old:raise RuntimeError('Principal is not registered')
        current=m.read_file(config,user.pw_uid) if config.exists() else ''
        if a.action=='enroll' and current and not a.import_legacy:raise RuntimeError('Existing config requires --import-legacy')
        port=old['port'] if old else a.port
        if port is None or not 1024<=port<=65535:raise RuntimeError('Specify a dedicated port from 1024..65535')
        service=m.read_file(base/'.env',0)
        if m.setting(service,'AGENT_SESSION_RELAY_ENABLED',False)=='true' and str(port)==(m.setting(service,'AGENT_SESSION_RELAY_PORT',False) or '47652'):raise RuntimeError('Port is used by legacy relay')
        for other in registry.glob('*.json'):
            if other!=descriptor and json.loads(m.read_file(other,0))['port']==port:raise RuntimeError('Port already assigned')
        secret=secrets.token_hex(32) if a.action in ('enroll','rotate','revoke') else old['key']
        enabled=a.action=='activate' or (a.action in ('rotate','repair') and old['enabled'])
        settings={'AGENT_MODE':'session','AGENT_MULTI_SESSION':'true','AGENT_SESSION_RELAY_PORT':port,'AGENT_SESSION_RELAY_KEY':secret}
        if m.setting(current,'AGENT_OFFLINE_CACHE_KEY',False) is None:
            settings['AGENT_OFFLINE_CACHE_KEY']=m.setting(current,'AGENT_SESSION_RELAY_KEY',False) or secrets.token_hex(32)
        # Existing cache setting is not reserialized: preserve literal custom key representation.
        plan={'version':1,'uid':user.pw_uid,'user':user.pw_name,'enabled':enabled,
              'service':m.enrollment.update_env(service,{'AGENT_SESSION_REGISTRY':str(registry), **({'AGENT_SESSION_RELAY_ENABLED':'false'} if a.import_legacy else {})}),
              'session':m.enrollment.update_env(current,settings),
              'descriptor':{'principal':'uid:'+str(user.pw_uid),'port':port,'key':secret,'enabled':enabled}}
        m.enrollment.atomic_write(journal,json.dumps(plan)+'\n',0,0,0o600)
    m.stopped(user)
    directory=base/'sessions'/str(user.pw_uid)
    m.enrollment.root_directory(base/'sessions');m.enrollment.root_directory(directory)
    m.enrollment.atomic_write(directory/'session.env',plan['session'],user.pw_uid,user.pw_gid,0o600)
    m.enrollment.atomic_write(registry/(str(user.pw_uid)+'.json'),json.dumps(plan['descriptor'])+'\n',0,gid,0o640)
    m.enrollment.atomic_write(base/'.env',plan['service'],0,gid,0o640)
    # A protected flag controls the system-wide XDG launcher; no session starts here.
    flag=directory/'multi-autostart.enabled'
    if plan['enabled']:m.enrollment.atomic_write(flag,'enabled\n',0,0,0o644)
    else:
        if flag.is_symlink():raise RuntimeError('Symlink flag rejected')
        flag.unlink(missing_ok=True)
    journal.unlink()
    fd=os.open(base,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)
    print('Principal updated. Processes remain stopped; identity, custom cache key and outbox retained.')
if __name__=='__main__':main()
