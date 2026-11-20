#!/usr/bin/env python3
"""Recover authenticated DB/configuration to NEW isolated resources; never promote."""
import argparse
import datetime
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import time
import uuid
from backup import FILES, private_directory, trusted_parents, stop, deadline
from verify_backup import verify


def run(*args, input=None, timeout=30):
    return subprocess.run(args, input=input, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                          timeout=timeout, check=True).stdout


def extract_configuration(ciphertext, identity, destination):
    process = subprocess.Popen(['age','--decrypt','-i',str(identity),str(ciphertext)],
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL)
    timer = deadline(process, 600)
    total = 0
    seen = set()
    try:
        with tarfile.open(fileobj=process.stdout, mode='r|') as archive:
            for member in archive:
                name = PurePosixPath(member.name)
                if (not member.isfile() or name.is_absolute() or '..' in name.parts
                    or not name.parts or name.parts[0] not in ('config','deployment')
                    or name.as_posix() in seen or member.size > 16*1024**2):
                    raise ValueError('Unsafe or duplicate archive entry')
                total += member.size
                if total > 64*1024**2: raise ValueError('Configuration exceeds recovery limit')
                seen.add(name.as_posix())
                path = destination.joinpath(*name.parts)
                path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                with path.open('xb') as output:
                    stream = archive.extractfile(member)
                    for chunk in iter(lambda: stream.read(1024*1024), b''): output.write(chunk)
                    output.flush(); os.fsync(output.fileno())
        while process.stdout.read(1024*1024): pass
        if process.wait(timeout=30): raise RuntimeError('Configuration authentication failed')
    finally:
        timer.cancel(); process.stdout.close(); stop(process)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--backup-dir', required=True, type=Path)
    p.add_argument('--identity-file', required=True, type=Path)
    p.add_argument('--output-dir', required=True, type=Path, help='Existing empty private root-owned directory')
    p.add_argument('--timeout-seconds', type=int, default=7200)
    a = p.parse_args()
    if os.name != 'posix' or os.geteuid() != 0: p.error('Run as root on an isolated Linux recovery host')
    if not 60 <= a.timeout_seconds <= 21600: p.error('Timeout must be 60..21600 seconds')
    for path in (a.backup_dir,a.identity_file,a.output_dir):
        if not path.is_absolute(): p.error('Use absolute paths')
        trusted_parents(path)
    private_directory(a.output_dir)
    if any(a.output_dir.iterdir()): p.error('Recovery output must be empty; nothing is overwritten')
    os.umask(0o077)
    started = time.monotonic()
    manifest = verify(a.backup_dir, a.identity_file)
    image = manifest.get('postgres_image_id','')
    if not re.fullmatch(r'sha256:[0-9a-f]{64}', image): raise ValueError('Missing exact original image ID')
    # No pull or upgrade during recovery. DevOps must retrieve the approved original image first.
    if run('docker','image','inspect','--format','{{.Id}}',image).decode().strip() != image:
        raise ValueError('Original PostgreSQL image is required locally')
    recovery_id = uuid.uuid4().hex
    container = 'monitor-recovery-'+recovery_id
    volume = container+'-data'
    receipt = {'recovery_id': recovery_id, 'source_commit': manifest.get('commit'),
               'backup_started_at': manifest.get('started_at'), 'image_id': image,
               'container': container, 'volume': volume, 'status': 'incomplete',
               'application_started': False, 'production_approved': False}
    receipt_path = a.output_dir/'recovery.json'
    def save():
        with (a.output_dir/'recovery.json.partial').open('w') as output:
            json.dump(receipt,output,indent=2); output.flush(); os.fsync(output.fileno())
        os.replace(a.output_dir/'recovery.json.partial',receipt_path)
    save()
    created = []
    try:
        extract_configuration(a.backup_dir/FILES[1],a.identity_file,a.output_dir)
        roles = a.output_dir/'config/init-roles.sql'
        users = a.output_dir/'config/accounts/users.json'
        if not roles.is_file() or not users.is_file(): raise ValueError('Required recovery configuration missing')
        json.loads(users.read_text())  # Validate structure without printing account data.
        live_definitions = a.output_dir/'config/rabbit-live-definitions.json'
        receipt['broker_live_definitions_available'] = live_definitions.is_file()
        if live_definitions.is_file():
            definitions = json.loads(live_definitions.read_text())
            receipt['broker_definition_counts'] = {name: len(definitions[name]) for name in ('users','vhosts','permissions','queues','exchanges','bindings')}
        receipt['broker_messages_restored'] = False
        save()
        run('docker','volume','create','--label','monitor.recovery='+recovery_id,volume)
        run('docker','create','--name',container,'--label','monitor.recovery='+recovery_id,
            '--network','none','--restart','no','--mount','type=volume,src='+volume+',dst=/var/lib/postgresql/data',
            '-e','POSTGRES_USER=monitor_bootstrap','-e','POSTGRES_DB=activity_monitor',
            '-e','POSTGRES_HOST_AUTH_METHOD=trust',image)
        created.append(container)
        run('docker','start',container)
        until = time.monotonic()+120
        while True:
            try:
                # The entrypoint's temporary initialization server accepts socket
                # connections but not TCP. Wait for the final server instead.
                run('docker','exec',container,'pg_isready','-h','127.0.0.1','-U','monitor_bootstrap','-d','activity_monitor',timeout=10)
                break
            except subprocess.SubprocessError:
                if time.monotonic()>until: raise RuntimeError('Recovery DB did not become ready')
                time.sleep(1)
        psql = ['docker','exec','-i',container,'psql','-X','-qAt','-v','ON_ERROR_STOP=1','-U','monitor_bootstrap','-d','activity_monitor']
        run(*psql,input=roles.read_bytes())
        available = run(*psql,input=b"SELECT count(*) FROM pg_available_extensions WHERE name='timescaledb';").strip()==b'1'
        if available:
            run(*psql,input=b'CREATE EXTENSION IF NOT EXISTS timescaledb; SELECT timescaledb_pre_restore();')
        decrypt = subprocess.Popen(['age','--decrypt','-i',str(a.identity_file),str(a.backup_dir/FILES[0])],
                                   stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,stdin=subprocess.DEVNULL)
        timer = deadline(decrypt,a.timeout_seconds)
        try:
            # Private diagnostics remain with the recovery, never in shared logs.
            with (a.output_dir/'restore.private.log').open('xb') as diagnostics:
                result = subprocess.run(['docker','exec','-i',container,'pg_restore','--exit-on-error',
                                         '-U','monitor_bootstrap','-d','activity_monitor'],stdin=decrypt.stdout,
                                        stdout=subprocess.DEVNULL,stderr=diagnostics,timeout=a.timeout_seconds)
            decrypt.stdout.close()
            if decrypt.wait(timeout=30) or result.returncode: raise RuntimeError('Database restore failed')
        finally:
            timer.cancel()
            decrypt.stdout.close()
            stop(decrypt)
        if available: run(*psql,input=b'SELECT timescaledb_post_restore();')
        # Never reactivate browser sessions from an older backup automatically.
        run(*psql,input=b'UPDATE human_sessions SET revoked_at=now() WHERE revoked_at IS NULL;')
        counts = run(*psql,input=b"SELECT json_build_object('devices',(SELECT count(*) FROM devices),'agent_credentials',(SELECT count(*) FROM agent_credentials),'policies',(SELECT count(*) FROM administration_policy),'processed_events',(SELECT count(*) FROM processed_events),'migrations',(SELECT count(*) FROM _sqlx_migrations),'activity_rows',(SELECT count(*) FROM activity_logs),'resource_rows',(SELECT count(*) FROM node_resource_metrics));",timeout=120)
        receipt.update(status='database_and_configuration_restored',counts=json.loads(counts),
                       human_sessions_revoked=True,elapsed_seconds=round(time.monotonic()-started,2),
                       recovery_recorded_at=datetime.datetime.now(datetime.timezone.utc).isoformat())
        invariants = json.loads(run(*psql,input=b"SELECT json_build_object('migrations_successful',NOT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE NOT success),'credential_orphans',(SELECT count(*) FROM agent_credentials c LEFT JOIN devices d ON d.device_id=c.device_id WHERE d.device_id IS NULL),'privileged_application_roles',(SELECT count(*) FROM pg_roles WHERE rolname IN ('monitor_app','monitor_maintenance') AND (rolsuper OR rolcreaterole OR rolcreatedb)),'active_human_sessions',(SELECT count(*) FROM human_sessions WHERE revoked_at IS NULL));"))
        receipt['invariants'] = invariants
        if (not invariants['migrations_successful'] or invariants['credential_orphans']
            or invariants['privileged_application_roles'] or invariants['active_human_sessions']):
            receipt['status'] = 'invariants_failed'; save()
            raise RuntimeError('Recovery invariants failed')
        if live_definitions.is_file():
            broker_image = manifest.get('rabbitmq_image_id','')
            if not re.fullmatch(r'sha256:[0-9a-f]{64}',broker_image): raise ValueError('Missing original broker image')
            if run('docker','image','inspect','--format','{{.Id}}',broker_image).decode().strip()!=broker_image:
                raise ValueError('Original broker image must be available locally')
            broker = container+'-broker'
            broker_volume = broker+'-data'
            bootstrap = 'restore_'+uuid.uuid4().hex
            receipt.update(broker_container=broker,broker_volume=broker_volume,broker_image_id=broker_image,
                           status='broker_recovery_incomplete')
            save()
            run('docker','volume','create','--label','monitor.recovery='+recovery_id,broker_volume)
            run('docker','create','--name',broker,'--hostname',broker,'--add-host',broker+':127.0.0.1','--label','monitor.recovery='+recovery_id,
                '--network','none','--restart','no','--mount','type=volume,src='+broker_volume+',dst=/var/lib/rabbitmq',
                '-e','RABBITMQ_DEFAULT_USER='+bootstrap,'-e','RABBITMQ_DEFAULT_PASS='+uuid.uuid4().hex+uuid.uuid4().hex,broker_image)
            created.append(broker)
            run('docker','start',broker)
            until = time.monotonic()+180
            while True:
                try:
                    run('docker','exec','--user','rabbitmq',broker,'rabbitmq-diagnostics','-q','-t','30','check_running',timeout=45)
                    break
                except subprocess.SubprocessError:
                    if time.monotonic()>until: raise RuntimeError('Recovery broker did not become ready')
                    time.sleep(1)
            target = '/tmp/monitor-recovery-definitions.json'
            run('docker','cp',str(live_definitions),broker+':'+target)
            run('docker','exec','--user','root',broker,'chown','rabbitmq:rabbitmq',target)
            run('docker','exec','--user','root',broker,'chmod','0600',target)
            run('docker','exec','--user','rabbitmq',broker,'rabbitmqctl','-q','import_definitions',target,timeout=120)
            run('docker','exec','--user','rabbitmq',broker,'rabbitmqctl','-q','delete_user',bootstrap)
            if '/' not in {v['name'] for v in definitions['vhosts']}:
                run('docker','exec','--user','rabbitmq',broker,'rabbitmqctl','-q','delete_vhost','/')
            run('docker','exec','--user','root',broker,'rm','--',target)
            receipt.update(status='database_configuration_and_broker_definitions_restored',broker_definitions_imported=True)
        receipt['elapsed_seconds'] = round(time.monotonic()-started,2)
        save()
    finally:
        stop_failed = []
        for resource in reversed(created):
            try: run('docker','stop','--time','15',resource,timeout=30)
            except (OSError,subprocess.SubprocessError): stop_failed.append(resource)
        if stop_failed:
            receipt['stop_failed']=stop_failed; save()
            raise RuntimeError('Recovery container stop failed; isolate before proceeding')
    print('Recovered to isolated stopped resources. Inspect private recovery.json; application/broker/cutover acceptance remains required.')


if __name__ == '__main__':
    try: main()
    except Exception:
        raise SystemExit('Isolated recovery failed. Preserve the private output and named recovery resources for investigation; no existing deployment was targeted.')
