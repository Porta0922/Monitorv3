#!/usr/bin/env python3
"""Prepare private candidate configuration on Linux. Never starts services or overwrites configuration."""
import argparse,base64,hashlib,json,os,secrets,shutil,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--users-file',type=Path,required=True,help='Managed accounts JSON with Argon2id hashes')
 parser.add_argument('--tls-dir',type=Path,required=True,help='Existing CA and issued web/broker certificates and keys')
 parser.add_argument('--web-host',required=True)
 parser.add_argument('--broker-host',required=True)
 parser.add_argument('--broker-port',type=int,default=25671)
 parser.add_argument('--web-port',type=int,default=18443)
 args=parser.parse_args()
 if not 1<=args.web_port<=65535:parser.error("web-port must be 1..65535")
 if not 1<=args.broker_port<=65535:parser.error("broker-port must be 1..65535")
 if os.name!='posix':parser.error('Run on the Linux deployment host to enforce POSIX permissions; no files created')
 accounts=json.loads(args.users_file.read_text(encoding='utf-8'))
 if not isinstance(accounts,list) or not accounts or not all(isinstance(a,dict) and str(a.get('password_hash','')).startswith('$argon2id$') for a in accounts):parser.error('Expected managed accounts with Argon2id hashes')
 if any(a.get('role')=='admin' and not a.get('disabled',False) and not a.get('totp_secret') for a in accounts):parser.error('Enabled administrators require a TOTP secret')
 if not any(a.get('role')=='admin' and not a.get('disabled',False) for a in accounts):parser.error('An enabled administrator is required')
 tls=args.tls_dir.resolve()
 def openssl(*parts):
  result=subprocess.run(['openssl',*parts],capture_output=True)
  if result.returncode:parser.error('TLS validation failed; check certificate chain, hostname, expiration and key (no secrets printed)')
  return result.stdout
 for prefix,hostname in [('web',args.web_host),('broker',args.broker_host)]:
  cert,key=tls/(prefix+'.crt'),tls/(prefix+'.key')
  openssl('x509','-in',str(cert),'-checkend','86400','-noout')
  openssl('verify','-CAfile',str(tls/'ca.crt'),'-untrusted',str(cert),'-verify_hostname',hostname,str(cert))
  if openssl('x509','-in',str(cert),'-pubkey','-noout').strip()!=openssl('pkey','-in',str(key),'-passin','pass:','-pubout').strip():parser.error('TLS certificate and private key differ')
 openssl('verify','-CAfile',str(tls/'ca.crt'),'-untrusted',str(tls/'broker.crt'),'-verify_hostname','rabbitmq',str(tls/'broker.crt'))
 target=ROOT/'.production'
 if target.exists():parser.error('.production already exists; rotate explicitly, do not regenerate existing-volume credentials')
 os.umask(0o077)
 target.mkdir(mode=0o700)
 (target/'secrets').mkdir(mode=0o700);(target/'accounts').mkdir(mode=0o700)
 bootstrap,owner,app,maintenance,jwt,broker,broker_admin=[secrets.token_hex(32) for _ in range(7)]
 values={'broker_admin_password':broker_admin,'db_password':bootstrap,'maintenance_password':maintenance,'jwt_secret':jwt,
 'database_url':f'postgresql://monitor_app:{app}@postgres:5432/activity_monitor',
 'migration_url':f'postgresql://monitor_owner:{owner}@postgres:5432/activity_monitor',
 'rabbitmq_url':f'amqps://monitor_server:{broker}@rabbitmq:5671/%2f'}
 for name,value in values.items():
  with (target/'secrets'/name).open('x') as f:f.write(value+'\n')
  # Individual file bind mounts need to be readable by service UID 65532.
  # Host traversal is blocked by .production and secrets (0700).
  (target/'secrets'/name).chmod(0o444)
 (target/'accounts/users.json').write_text(json.dumps(accounts,indent=2)+'\n',encoding='utf-8')
 (target/'accounts/users.json').chmod(0o444);(target/'accounts').chmod(0o555)
 init="REVOKE ALL ON DATABASE activity_monitor FROM PUBLIC;\nREVOKE ALL ON SCHEMA public FROM PUBLIC;\n"
 for role,password in [('monitor_owner',owner),('monitor_app',app),('monitor_maintenance',maintenance)]:
  init+=f"CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD '{password}';\nGRANT CONNECT ON DATABASE activity_monitor TO {role};\n"
 init+="ALTER DATABASE activity_monitor OWNER TO monitor_owner;\nALTER SCHEMA public OWNER TO monitor_owner;\n"
 (target/'init-roles.sql').write_text(init,encoding='utf-8');(target/'init-roles.sql').chmod(0o444)
 (target/'tls').mkdir(mode=0o700)
 for name in ['ca.crt','web.crt','web.key','broker.crt','broker.key']:
  shutil.copyfile(tls/name,target/'tls'/name);(target/'tls'/name).chmod(0o444)
 def rabbit_hash(password):
  salt=secrets.token_bytes(4)
  return base64.b64encode(salt+hashlib.sha256(salt+password.encode()).digest()).decode()
 password_hash=rabbit_hash(broker)
 definitions={'users':[{'name':'monitor_server','password_hash':password_hash,'hashing_algorithm':'rabbit_password_hashing_sha256','tags':[]},{'name':'monitor_identity','password_hash':rabbit_hash(broker_admin),'hashing_algorithm':'rabbit_password_hashing_sha256','tags':['administrator']}],'vhosts':[{'name':'/'}],'permissions':[{'user':'monitor_server','vhost':'/','configure':r'^(monitoring|(activity|inventory|heartbeat|usb|wifi|running_apps|security)_queue(\.dead)?)$','write':r'^(monitoring|amq\.default|(activity|inventory|heartbeat|usb|wifi|running_apps|security)_queue(\.dead)?)$','read':r'^(monitoring|(activity|inventory|heartbeat|usb|wifi|running_apps|security)_queue(\.dead)?)$'}]}
 definitions['policies']=[{'vhost':'/','name':'bounded-telemetry','pattern':r'^(activity|inventory|heartbeat|usb|wifi|running_apps|security)_queue(\.dead)?$','apply-to':'queues','priority':10,'definition':{'max-length-bytes':67108864,'overflow':'reject-publish'}}]
 # RabbitMQ's non-root process needs to read these two files. The private parent
 # directory still denies host traversal to other users; bind mounts expose only these files.
 (target/'rabbit-definitions.json').write_text(json.dumps(definitions)+'\n',encoding='utf-8')
 (target/'rabbitmq.conf').write_text('definitions.import_backend = local_filesystem\ndefinitions.local.path = /etc/rabbitmq/definitions.json\nlisteners.tcp = none\nlisteners.ssl.default = 5671\nssl_options.cacertfile = /run/tls/ca.crt\nssl_options.certfile = /run/tls/broker.crt\nssl_options.keyfile = /run/tls/broker.key\nssl_options.verify = verify_peer\nssl_options.fail_if_no_peer_cert = false\nssl_options.versions.1 = tlsv1.3\nssl_options.versions.2 = tlsv1.2\nmanagement.ssl.port = 15671\nmanagement.ssl.cacertfile = /run/tls/ca.crt\nmanagement.ssl.certfile = /run/tls/broker.crt\nmanagement.ssl.keyfile = /run/tls/broker.key\nmanagement.ssl.versions.1 = tlsv1.3\nmanagement.ssl.versions.2 = tlsv1.2\nmax_message_size = 1048576\nchannel_max = 16\n',encoding='utf-8')
 for name in ['rabbit-definitions.json','rabbitmq.conf']:(target/name).chmod(0o444)
 (target/'deployment.json').write_text(json.dumps({'web_host':args.web_host,'broker_host':args.broker_host})+'\n',encoding='utf-8')
 (target/'endpoint.env').write_text(f'AGENT_BROKER_URL=amqps://{args.broker_host}:{args.broker_port}/%2F\nMONITOR_PUBLIC_ORIGIN=https://{args.web_host}'+('' if args.web_port==443 else f':{args.web_port}')+'\n',encoding='utf-8')
 (target/'endpoint.env').chmod(0o600)
 print('Private candidate configuration created. No services started. Keep .production out of backups without encryption and out of source control.')
if __name__=='__main__':main()
