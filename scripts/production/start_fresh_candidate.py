#!/usr/bin/env python3
"""Guarded first startup of the next image candidate; never migrate existing volumes."""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]

def output(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True, stderr=subprocess.DEVNULL, timeout=30).strip()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--project', required=True)
    p.add_argument('--env-file', required=True, type=Path)
    p.add_argument('--start', action='store_true', help='Actually build/start after all freshness checks; otherwise inspect only')
    a = p.parse_args()
    if not re.fullmatch(r'monitor-next-[a-z0-9][a-z0-9-]{0,35}',a.project): p.error('Use a new monitor-next-* project')
    if not a.env_file.is_absolute() or not a.env_file.is_file(): p.error('Provide the protected deployment env file as an absolute path')
    if output('docker','ps','-aq','--filter','label=com.docker.compose.project='+a.project):
        p.error('Project already has containers; first-start helper will not alter it')
    if output('docker','volume','ls','-q','--filter','label=com.docker.compose.project='+a.project):
        p.error('Project already has volumes; preserve and inspect them')
    # Also reject a pre-existing volume with our generated name but missing labels.
    existing = set(output('docker','volume','ls','-q').splitlines())
    if any(a.project+'_'+name in existing for name in ('postgres17_data','rabbitmq42_data')):
        p.error('Candidate data volumes already exist')
    command = ['docker','compose','--project-name',a.project,'--env-file',str(a.env_file)]
    for name in ('docker-compose.yml','docker-compose.production.yml','docker-compose.next.yml'):
        command += ['-f',str(ROOT/name)]
    config = json.loads(output(*command,'config','--format','json'))
    for service, volume, destination in (('postgres','postgres17_data','/var/lib/postgresql/data'),('rabbitmq','rabbitmq42_data','/var/lib/rabbitmq')):
        mounts = [m for m in config['services'][service]['volumes'] if m['target']==destination]
        if len(mounts)!=1 or mounts[0].get('type')!='volume' or mounts[0]['source']!=volume:
            p.error('Unexpected candidate data mount')
        declared = config['volumes'][volume]
        if declared.get('external') or declared.get('name') != a.project+'_'+volume:
            p.error('External or unexpected volume names are forbidden')
    if not a.start:
        print('Fresh candidate configuration inspected; nothing built, started or migrated.')
        return
    subprocess.run(command+['up','-d','--build','--wait','--wait-timeout','180'],cwd=ROOT,check=True)
    print('Fresh candidate started. Acceptance and data migration remain separate; no existing project was targeted.')

if __name__ == '__main__':
    try: main()
    except Exception: raise SystemExit('Candidate preparation failed; preserve existing resources. No configuration secrets printed.')
