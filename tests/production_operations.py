"""Isolated Linux/root recovery and watchdog regression. Synthetic data only."""
import hashlib
import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import uuid

ROOT=Path('/workspace')
sys.path.insert(0,str(ROOT/'scripts/production'))
import backup
import notify_failure as alerts
import watch_health

OLD='timescale/timescaledb:latest-pg15@sha256:6343bdc87ca132c6b53acb26113a6bad1821d188fa39975a745f32ddd9757634'
BRIDGE='timescale/timescaledb:2.28.3-pg17@sha256:9372c578c11ea83c56e4a8f7af6ec4156646722d270f5ade79559ccb101161a9'
BROKER='rabbitmq:3.12-management-alpine@sha256:0b44fbcc3a4bf22d00090f1353127577dbe1fcb109c41669733a9d7ecf6c3a78'

def run(*args,input=None,timeout=120):
    r=subprocess.run(args,input=input,capture_output=True,timeout=timeout)
    if r.returncode: raise RuntimeError(str(args[:3])+': '+r.stderr.decode(errors='replace')[-1800:])
    return r.stdout

def wait(fn):
    for _ in range(90):
        try: return fn()
        except (RuntimeError,subprocess.SubprocessError): time.sleep(1)
    raise RuntimeError('Fixture readiness failed')

def main():
    os.umask(0o077)
    base=Path('/var/lib/monitor-fixture');base.mkdir(mode=0o700,exist_ok=True)
    work=Path(tempfile.mkdtemp(dir=base))
    project='monitor-ops-'+uuid.uuid4().hex[:10]
    compose=work/'compose.json'
    spec={'services':{'postgres':{'image':OLD,'environment':{'POSTGRES_USER':'monitor_bootstrap','POSTGRES_DB':'activity_monitor','POSTGRES_HOST_AUTH_METHOD':'trust'}},
                       'rabbitmq':{'image':BROKER}},'networks':{'default':{'internal':True}}}
    for name in ('server','dashboard','db-maintenance'):
        spec['services'][name]={'image':OLD,'entrypoint':['/bin/sh','-c','sleep 1800']}
    compose.write_text(json.dumps(spec))
    cmd=['docker','compose','-p',project,'-f',str(compose)]
    recovery=work/'restored';recovery.mkdir(mode=0o700)
    extras=[];volumes=[];receiver=None
    def sql(container,statement):
        return run('docker','exec','-i',container,'psql','-h','127.0.0.1','-X','-qAt','-U','monitor_bootstrap','-d','activity_monitor','-v','ON_ERROR_STOP=1',input=statement.encode())
    try:
        run(*cmd,'up','-d',timeout=180)
        pg=run(*cmd,'ps','-q','postgres').decode().strip()
        rabbit=run(*cmd,'ps','-q','rabbitmq').decode().strip()
        wait(lambda:sql(pg,'SELECT 1;'))
        wait(lambda:run('docker','exec',rabbit,'rabbitmq-diagnostics','-q','check_running'))
        roles=''.join('CREATE ROLE '+role+' LOGIN;\n' for role in ('monitor_owner','monitor_app','monitor_maintenance'))
        sql(pg,roles)
        for migration in sorted((ROOT/'server/migrations').glob('*.sql')):
            sql(pg,'BEGIN;\n'+migration.read_text()+'\nCOMMIT;')
        sql(pg,"CREATE TABLE _sqlx_migrations(version bigint PRIMARY KEY,success boolean NOT NULL); INSERT INTO _sqlx_migrations VALUES(17,true);")
        sql(pg,"""
        CREATE EXTENSION IF NOT EXISTS timescaledb;
        CREATE TABLE recovery_evidence(at timestamptz NOT NULL, device integer, value text);
        SELECT create_hypertable('recovery_evidence','at',chunk_time_interval=>interval '1 day');
        INSERT INTO recovery_evidence SELECT '2020-01-01'::timestamptz+n*interval '1 minute',n%3,md5(n::text) FROM generate_series(1,5000)n;
        ALTER TABLE recovery_evidence SET(timescaledb.compress,timescaledb.compress_segmentby='device');
        SELECT compress_chunk(show_chunks('recovery_evidence'));
        INSERT INTO devices(device_id,hostname,last_seen) VALUES('00000000-0000-0000-0000-000000000001','SYNTHETIC',now());
        INSERT INTO agent_credentials(credential_id,device_id,token_hash,expires_at) VALUES('00000000-0000-0000-0000-000000000002','00000000-0000-0000-0000-000000000001',repeat('a',64),now()+interval '1 day');
        INSERT INTO administration_policy(scope,policy,updated_by) VALUES('global','{}','fixture');
        INSERT INTO human_sessions(session_id,token_hash,username,issued_role,account_version,expires_at) VALUES('00000000-0000-0000-0000-000000000003',repeat('b',64),'fixture','viewer',repeat('c',64),now()+interval '1 day');
        INSERT INTO maintenance_status VALUES('metrics-retention',now(),0,0);
        """)
        evidence="COPY (SELECT * FROM recovery_evidence ORDER BY at) TO STDOUT;"
        expected=hashlib.sha256(sql(pg,evidence)).hexdigest()
        identity=work/'identity';recipients=work/'recipients'
        run('age-keygen','-o',str(identity));recipients.write_bytes(run('age-keygen','-y',str(identity)))
        config=work/'config';config.mkdir(mode=0o700)
        (config/'init-roles.sql').write_text(roles)
        (config/'users.json').write_text('[{"username":"fixture","role":"admin"}]')
        archived=work/'backup';archived.mkdir(mode=0o700)
        backup.encrypt_dump(cmd+['exec','-T','postgres','pg_dump','-U','monitor_bootstrap','-d','activity_monitor','-Fc'],recipients,archived/backup.FILES[0],120)
        definitions=backup.live_broker_definitions(cmd)
        backup.encrypt_configuration({'config/init-roles.sql':config/'init-roles.sql','config/accounts/users.json':config/'users.json'},recipients,archived/backup.FILES[1],120,{'config/rabbit-live-definitions.json':definitions})
        manifest={'format':1,'status':'complete','postgres_image_id':run('docker','inspect','--format','{{.Image}}',pg).decode().strip(),
                  'rabbitmq_image_id':run('docker','inspect','--format','{{.Image}}',rabbit).decode().strip(),
                  'files':{n:{'sha256':backup.digest(archived/n),'bytes':(archived/n).stat().st_size} for n in backup.FILES}}
        (archived/'manifest.json').write_text(json.dumps(manifest))
        # Synthetic fixture only: retain the exception location for actionable failures.
        run('python3','-c',"import sys; sys.path.insert(0,'/workspace/scripts/production'); import restore_isolated; restore_isolated.main()",'--backup-dir',str(archived),'--identity-file',str(identity),'--output-dir',str(recovery),timeout=600)
        receipt=json.loads((recovery/'recovery.json').read_text())
        assert receipt['broker_definitions_imported'] and not receipt['broker_messages_restored']
        assert receipt['counts']['devices']==1 and receipt['counts']['agent_credentials']==1 and receipt['counts']['policies']==1
        restored=receipt['container']
        assert run('docker','inspect','--format','{{.State.Status}}',restored).strip()==b'exited'
        assert run('docker','inspect','--format','{{.HostConfig.NetworkMode}}',restored).strip()==b'none'
        run('docker','start',restored);wait(lambda:sql(restored,'SELECT 1;'))
        assert hashlib.sha256(sql(restored,evidence)).hexdigest()==expected
        assert sql(restored,'SELECT count(*) FROM human_sessions WHERE revoked_at IS NULL;').strip()==b'0'
        print('PASS: actual isolated DB/config/broker recovery; 5000 compressed rows identical; credentials/policies preserved; human sessions revoked',flush=True)
        # A separate major-upgrade rehearsal preserves extension 2.28.3 first.
        bridge=project+'-bridge';volume=bridge+'-data';extras.append(bridge);volumes.append(volume)
        run('docker','volume','create',volume)
        run('docker','run','-d','--name',bridge,'--network','none','-v',volume+':/var/lib/postgresql/data','-e','POSTGRES_USER=monitor_bootstrap','-e','POSTGRES_DB=activity_monitor','-e','POSTGRES_HOST_AUTH_METHOD=trust',BRIDGE)
        wait(lambda:sql(bridge,'SELECT 1;'))
        sql(bridge,roles)
        sql(bridge,'CREATE EXTENSION IF NOT EXISTS timescaledb; SELECT timescaledb_pre_restore();')
        dump=run(*cmd,'exec','-T','postgres','pg_dump','-U','monitor_bootstrap','-d','activity_monitor','-Fc')
        run('docker','exec','-i',bridge,'pg_restore','--exit-on-error','-U','monitor_bootstrap','-d','activity_monitor',input=dump)
        sql(bridge,'SELECT timescaledb_post_restore();')
        assert hashlib.sha256(sql(bridge,evidence)).hexdigest()==expected
        run('docker','stop',bridge);run('docker','rm',bridge);extras.remove(bridge)
        upgraded=project+'-upgraded';extras.append(upgraded)
        run('docker','run','-d','--name',upgraded,'--network','none','-v',volume+':/var/lib/postgresql/data','-e','POSTGRES_USER=monitor_bootstrap','-e','POSTGRES_DB=activity_monitor','-e','POSTGRES_HOST_AUTH_METHOD=trust','monitor-security-postgres:20261117')
        wait(lambda:sql(upgraded,'SELECT 1;'))
        sql(upgraded,'ALTER EXTENSION timescaledb UPDATE;')
        assert sql(upgraded,"SELECT extversion FROM pg_extension WHERE extname='timescaledb';").strip()==b'2.32.0'
        assert hashlib.sha256(sql(upgraded,evidence)).hexdigest()==expected
        print('PASS: PG15/Timescale2.28.3 -> PG17/2.28.3 -> patched PG17/2.32.0; compressed data preserved',flush=True)
        # Real local HTTPS receiver: payload, redirect rejection and CA rejection.
        cert=work/'ca.crt';key=work/'ca.key'
        run('openssl','req','-x509','-newkey','rsa:2048','-nodes','-keyout',str(key),'-out',str(cert),'-days','1','-subj','/CN=localhost','-addext','subjectAltName=DNS:localhost,IP:127.0.0.1')
        received=[]
        class Receiver(http.server.BaseHTTPRequestHandler):
            status=200
            def do_POST(self):
                received.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                self.send_response(self.status);self.send_header('Location','https://localhost:1/');self.end_headers()
            def log_message(self,*args):pass
        receiver=http.server.HTTPServer(('127.0.0.1',0),Receiver)
        context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);context.load_cert_chain(cert,key)
        receiver.socket=context.wrap_socket(receiver.socket,server_side=True)
        threading.Thread(target=receiver.serve_forever,daemon=True).start()
        config_path=work/'webhook.json';config_path.write_text(json.dumps({'url':f'https://127.0.0.1:{receiver.server_port}/','ca_file':str(cert)}))
        cfg,ctx=alerts.configuration(config_path)
        assert alerts.deliver(cfg,ctx,'fixture.service')
        Receiver.status=302;assert not alerts.deliver(cfg,ctx,'fixture.service');Receiver.status=200
        assert not alerts.deliver(cfg,ssl.create_default_context(),'fixture.service')
        # Exercise watchdog SQL and actual Docker state, plus notification cooldown.
        argv=['watch_health','--project',project,'--webhook-config',str(config_path),'--state-dir',str(work/'state'),'--disk-path','/','--min-free-bytes','1','--min-free-percent','1']
        sys.argv=argv
        old_queues=watch_health.QUEUES;watch_health.QUEUES=()  # queue-specific assertions follow independently
        assert watch_health.main()==0
        sql(pg,"INSERT INTO audit_events(actor,action,target) SELECT 'fixture','auth.login.failed','fixture' FROM generate_series(1,25);")
        before=len(received);assert watch_health.main()==1;assert len(received)==before+1
        assert watch_health.main()==1 and len(received)==before+1
        sql(pg,'DELETE FROM audit_events;');assert watch_health.main()==0 and received[-1]['findings']==[]
        watch_health.QUEUES=old_queues
        print('PASS: HTTPS delivery, redirect/untrusted-CA rejection, auth detection, cooldown and recovery',flush=True)
    except Exception:
        # This fixture contains no real accounts or production data.
        diagnostic = recovery/'restore.private.log'
        if diagnostic.exists(): print(diagnostic.read_text()[-4000:], file=sys.stderr)
        raise
    finally:
        if receiver:receiver.shutdown();receiver.server_close()
        if (recovery/'recovery.json').exists():
            receipt=json.loads((recovery/'recovery.json').read_text());prefix='monitor-recovery-'+receipt['recovery_id']
            for field in ('container','broker_container'):
                name=receipt.get(field)
                if name and name.startswith(prefix):subprocess.run(['docker','rm','-fv',name],capture_output=True,timeout=60)
            for field in ('volume','broker_volume'):
                name=receipt.get(field)
                if name and name.startswith(prefix):subprocess.run(['docker','volume','rm',name],capture_output=True,timeout=60)
        for name in extras:subprocess.run(['docker','rm','-fv',name],capture_output=True,timeout=60)
        for name in volumes:subprocess.run(['docker','volume','rm',name],capture_output=True,timeout=60)
        subprocess.run(cmd+['down','--volumes'],capture_output=True,timeout=120)

if __name__=='__main__':main()
