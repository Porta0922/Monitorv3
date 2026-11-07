"""Isolated HTTP/AMQPS identity regression. Needs Docker, built server/probe, cryptography.
Uses fresh disposable DB/broker and synthetic certificates; never touches the pilot.
"""
import http.server, threading, sys
import base64, datetime, hashlib, hmac, ipaddress, json, os, secrets, socket, ssl, subprocess, tempfile, time, urllib.request, urllib.error, uuid
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID, ExtendedKeyUsageOID
ROOT=Path(__file__).resolve().parents[1]
BROKER_IMAGE=os.environ.get('TEST_RABBITMQ_IMAGE','rabbitmq:3.12-management-alpine@sha256:0b44fbcc3a4bf22d00090f1353127577dbe1fcb109c41669733a9d7ecf6c3a78')
SUFFIX='.exe' if os.name=='nt' else ''
SERVER=ROOT/'target/debug'/('activity-monitor-server'+SUFFIX)
PROBE=ROOT/'target/debug'/('agent-transport-probe'+SUFFIX)
TAG=secrets.token_hex(4);PG='monitor-identity-db-'+TAG;BROKER='monitor-identity-broker-'+TAG
process=None
proxy=None

def run(args,**kwargs):
 r=subprocess.run(args,capture_output=True,text=True,**kwargs)
 if r.returncode:raise RuntimeError('Command failed: '+str(args[0])+' '+r.stderr[-1500:])
 return r.stdout.strip()
def docker(*args,**kwargs):return run(['docker',*args],**kwargs)
def sql(statement):return docker('exec','-i',PG,'psql','-X','-tA','-U','postgres','-d','activity_monitor','-v','ON_ERROR_STOP=1',input=statement)
def port(name,target):return int(docker('port',name,str(target)+'/tcp').rsplit(':',1)[1])
def wait_for(fn):
 for _ in range(60):
  try:
   result=fn()
   if result is not False:return result
  except (RuntimeError,OSError,urllib.error.HTTPError) as error:last_error=str(error)
  time.sleep(1)
 raise RuntimeError('Readiness timeout: '+locals().get('last_error','condition not reached'))
def certs(path):
 now=datetime.datetime.now(datetime.timezone.utc)
 key=rsa.generate_private_key(public_exponent=65537,key_size=2048)
 name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'Monitor synthetic test CA')])
 ca=x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key()).serial_number(x509.random_serial_number()).not_valid_before(now-datetime.timedelta(minutes=1)).not_valid_after(now+datetime.timedelta(days=2)).add_extension(x509.BasicConstraints(ca=True,path_length=None),True).add_extension(x509.KeyUsage(digital_signature=True,content_commitment=False,key_encipherment=False,data_encipherment=False,key_agreement=False,key_cert_sign=True,crl_sign=True,encipher_only=None,decipher_only=None),True).add_extension(x509.SubjectKeyIdentifier.from_public_key(key.public_key()),False).sign(key,hashes.SHA256())
 leaf_key=rsa.generate_private_key(public_exponent=65537,key_size=2048)
 leaf=x509.CertificateBuilder().subject_name(x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'localhost')])).issuer_name(name).public_key(leaf_key.public_key()).serial_number(x509.random_serial_number()).not_valid_before(now-datetime.timedelta(minutes=1)).not_valid_after(now+datetime.timedelta(days=1)).add_extension(x509.SubjectAlternativeName([x509.DNSName('localhost'),x509.DNSName('rabbitmq'),x509.IPAddress(ipaddress.ip_address('127.0.0.1'))]),False).add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]),False).add_extension(x509.BasicConstraints(ca=False,path_length=None),True).add_extension(x509.SubjectKeyIdentifier.from_public_key(leaf_key.public_key()),False).add_extension(x509.AuthorityKeyIdentifier.from_issuer_public_key(key.public_key()),False).sign(key,hashes.SHA256())
 (path/'ca.crt').write_bytes(ca.public_bytes(serialization.Encoding.PEM))
 (path/'broker.crt').write_bytes(leaf.public_bytes(serialization.Encoding.PEM))
 (path/'broker.key').write_bytes(leaf_key.private_bytes(serialization.Encoding.PEM,serialization.PrivateFormat.PKCS8,serialization.NoEncryption()))
 for f in path.iterdir():f.chmod(0o644)
def b64(data):return base64.urlsafe_b64encode(data).decode().rstrip('=')
def jwt(secret,role='admin'):
 body=b64(json.dumps({'sub':'isolated-test','role':role,'iat':int(time.time()),'exp':int(time.time())+3600}).encode())
 message=b64(b'{"alg":"HS256","typ":"JWT"}')+'.'+body
 return message+'.'+b64(hmac.new(secret.encode(),message.encode(),hashlib.sha256).digest())

with tempfile.TemporaryDirectory(prefix='monitor-identity-') as tmp:
 path=Path(tmp);path.chmod(0o755);certs(path)
 bootstrap,owner,app,broker,admin,jwt_secret=[secrets.token_hex(32) for _ in range(6)]
 try:
  docker('run','--rm','-d','--name',PG,'-p','127.0.0.1::5432','-e','POSTGRES_PASSWORD='+bootstrap,'-e','POSTGRES_DB=activity_monitor','timescale/timescaledb:latest-pg15@sha256:6343bdc87ca132c6b53acb26113a6bad1821d188fa39975a745f32ddd9757634')
  wait_for(lambda:docker('exec',PG,'pg_isready','-h','127.0.0.1','-U','postgres'))
  sql("CREATE ROLE monitor_owner LOGIN PASSWORD '"+owner+"'; CREATE ROLE monitor_app LOGIN PASSWORD '"+app+"'; CREATE ROLE monitor_maintenance LOGIN; ALTER DATABASE activity_monitor OWNER TO monitor_owner; ALTER SCHEMA public OWNER TO monitor_owner;")
  pgport=port(PG,5432)
  migration=path/'migration';migration.write_text(f'postgresql://monitor_owner:{owner}@127.0.0.1:{pgport}/activity_monitor')
  env=os.environ.copy();env['MIGRATION_DATABASE_URL_FILE']=str(migration)
  print('Applying migrations in isolated DB',flush=True)
  run([str(SERVER),'--migrate'],env=env,timeout=60)
  print('Migrations complete',flush=True)
  def rabbit_user(name,password,tags):
   salt=secrets.token_bytes(4)
   return {'name':name,'password_hash':base64.b64encode(salt+hashlib.sha256(salt+password.encode()).digest()).decode(),'hashing_algorithm':'rabbit_password_hashing_sha256','tags':tags}
  definitions={'users':[rabbit_user('monitor_server',broker,[]),rabbit_user('monitor_identity',admin,['administrator'])],'vhosts':[{'name':'/'}],'permissions':[{'user':'monitor_server','vhost':'/','configure':'.*','write':'.*','read':'.*'}]}
  definitions['policies']=[{'vhost':'/','name':'bounded-telemetry','pattern':r'^(activity|inventory|heartbeat|usb|wifi|running_apps|security)_queue(\.dead)?$','apply-to':'queues','priority':10,'definition':{'max-length-bytes':67108864,'overflow':'reject-publish'}}]
  (path/'definitions.json').write_text(json.dumps(definitions))
  (path/'rabbitmq.conf').write_text('definitions.import_backend = local_filesystem\ndefinitions.local.path = /test/definitions.json\nlisteners.tcp = none\nlisteners.ssl.default = 5671\nssl_options.cacertfile = /test/ca.crt\nssl_options.certfile = /test/broker.crt\nssl_options.keyfile = /test/broker.key\nssl_options.verify = verify_peer\nssl_options.fail_if_no_peer_cert = false\nmanagement.ssl.port = 15671\nmanagement.ssl.cacertfile = /test/ca.crt\nmanagement.ssl.certfile = /test/broker.crt\nmanagement.ssl.keyfile = /test/broker.key\nmax_message_size = 1048576\nchannel_max = 16\n')
  docker('run','-d','--name',BROKER,'-p','127.0.0.1::5671','-p','127.0.0.1::15671','-v',str(path)+':/test:ro','-v',str(path/'rabbitmq.conf')+':/etc/rabbitmq/rabbitmq.conf:ro',BROKER_IMAGE)
  amqpport=port(BROKER,5671);adminport=port(BROKER,15671)
  context=ssl.create_default_context(cafile=str(path/'ca.crt'))
  def broker_api(resource):
   request=urllib.request.Request(f'https://127.0.0.1:{adminport}/api/'+resource,headers={'Authorization':'Basic '+base64.b64encode(('monitor_identity:'+admin).encode()).decode()})
   with urllib.request.urlopen(request,context=context,timeout=3) as response:return json.load(response)
  print('Waiting for isolated TLS broker',flush=True)
  def broker_ready():
   state=docker('inspect','--format','{{.State.Status}}',BROKER)
   if state=='exited':raise AssertionError('Isolated broker exited: '+docker('logs','--tail','100',BROKER))
   return broker_api('overview')
  wait_for(broker_ready)
  print('TLS broker ready',flush=True)
  for name,value in {'jwt':jwt_secret,'db':f'postgresql://monitor_app:{app}@127.0.0.1:{pgport}/activity_monitor','rabbit':f'amqps://monitor_server:{broker}@127.0.0.1:{amqpport}/%2F','admin':admin}.items():(path/name).write_text(value)
  password=secrets.token_hex(24)
  password_hash=run([str(SERVER),'--hash-password-stdin'],input=password)
  mfa_secret=base64.b32encode(secrets.token_bytes(20)).decode().rstrip('=')
  accounts=[{'username':name,'password_hash':password_hash,'role':role,'disabled':False,**({'totp_secret':mfa_secret} if role=='admin' else {})} for name,role in [('isolated-test','admin'),('viewer','viewer'),('operator','operator'),('rate-budget','viewer')]]
  (path/'users.json').write_text(json.dumps(accounts))
  with socket.socket() as sock:sock.bind(('127.0.0.1',0));httpport=sock.getsockname()[1]
  for key in ['JWT_SECRET','DATABASE_URL','RABBITMQ_URL','AGENT_AUTH_TOKEN','AGENT_AUTH_TOKEN_FILE','ADMIN_PASSWORD']:env.pop(key,None)
  env.update(MONITOR_ENV='production',MONITOR_PUBLIC_ORIGIN='https://localhost',SERVER_HOST='127.0.0.1',SERVER_PORT=str(httpport),JWT_SECRET_FILE=str(path/'jwt'),DATABASE_URL_FILE=str(path/'db'),RABBITMQ_URL_FILE=str(path/'rabbit'),AUTH_USERS_FILE=str(path/'users.json'),SSL_CERT_FILE=str(path/'ca.crt'),BROKER_ADMIN_URL=f'https://127.0.0.1:{adminport}',BROKER_ADMIN_PASSWORD_FILE=str(path/'admin'),AGENT_BROKER_URL=f'amqps://127.0.0.1:{amqpport}/%2F')
  log=(path/'server.log').open('w');process=subprocess.Popen([str(SERVER)],cwd=path,env=env,stdout=log,stderr=log)
  token={}
  def api(method,route,body=None,credential=None,human=token,expected=200):
   headers={'Content-Type':'application/json'}
   if isinstance(human,dict):headers.update(human)
   elif human:headers['Authorization']='Bearer '+human
   if credential:headers['x-agent-token']=credential
   request=urllib.request.Request(f'http://127.0.0.1:{httpport}'+route,data=json.dumps(body).encode() if body is not None else None,headers=headers,method=method)
   try:response=urllib.request.urlopen(request,timeout=70)
   except urllib.error.HTTPError as error:response=error
   with response:
    status=response.status;raw=response.read()
   if status!=expected:raise RuntimeError(f'{method} {route}: expected {expected}, got {status}')
   return (raw.decode() if response.headers.get('Content-Type','').startswith('text/csv') else json.loads(raw)) if raw else None
  print('Waiting for isolated server',flush=True)
  wait_for(lambda:api('GET','/health',human=None))
  def totp(secret):
   step=int(time.time())//30
   digest=hmac.new(base64.b32decode(secret+'='*((-len(secret))%8)),step.to_bytes(8,'big'),hashlib.sha1).digest();offset=digest[-1]&15
   return f'{(int.from_bytes(digest[offset:offset+4],"big")&0x7fffffff)%1000000:06}'
  def login(name,otp='',expected=200):
   request=urllib.request.Request(f'http://127.0.0.1:{httpport}/api/auth/login',data=json.dumps({'username':name,'password':password,'otp':otp}).encode(),headers={'Content-Type':'application/json','X-Monitor-Client':'dashboard'})
   try: response=urllib.request.urlopen(request,timeout=20)
   except urllib.error.HTTPError as error: response=error
   with response:
    assert response.status==expected,(name,response.status,expected)
    data=json.load(response)
    if expected!=200:return data
    cookie=response.headers['Set-Cookie'];assert all(v in cookie for v in ['HttpOnly','Secure','SameSite=Strict','Path=/'])
    assert 'token' not in data
    return {'Cookie':cookie.split(';')[0],'X-CSRF-Token':data['csrf_token']}
  login('isolated-test',expected=401)
  first_admin_code=totp(mfa_secret)
  token.update(login('isolated-test',first_admin_code))
  login('isolated-test',first_admin_code,expected=401)
  humans={role:login(role) for role in ['viewer','operator']}
  api('GET','/api/devices',human=jwt(jwt_secret),expected=401)
  exec((ROOT/'tests/human-session-checks.py').read_text(),globals())
  if '--human-only' in sys.argv:raise SystemExit(0)
  print('Server ready; checking enrollment',flush=True)
  device,other=str(uuid.uuid4()),str(uuid.uuid4())
  for id in [device,other]:api('POST','/api/devices/register',{'device_id':id,'hostname':'SYNTHETIC_IDENTITY_TEST'},expected=200)
  for role in ['viewer','operator']:api('POST','/api/agent-credentials/enrollment/'+device,{},human=humans[role],expected=403)
  invitation=api('POST','/api/agent-credentials/enrollment/'+device,{},expected=201)
  new_token=secrets.token_hex(32)
  body={'device_id':device,'code':invitation['code'],'new_token':new_token}
  identity=api('POST','/api/agent/enroll',body,human=None)
  print('Enrollment complete; checking authorization',flush=True)
  assert api('POST','/api/agent/enroll',body,human=None)['credential_id']==identity['credential_id']
  api('POST','/api/agent/enroll',{**body,'new_token':secrets.token_hex(32)},human=None,expected=401)
  api('GET','/api/agent/policy?device_id='+other,credential=new_token,human=None,expected=403)
  api('POST','/api/logs/ingest',{'device_id':other,'events':[]},credential=new_token,human=None,expected=403)
  api('GET','/api/agent/policy?device_id='+device,credential=new_token,human=None)
  api('GET','/api/agent/commands?device_id='+other,credential=new_token,human=None,expected=403)
  api('GET','/api/devices',credential=new_token,human=None,expected=401)
  command=api('POST','/api/agent/commands',{'device_id':other,'command':'ping'},expected=201)
  api('POST','/api/agent/commands/'+command['command_id']+'/ack',{'status':'completed'},credential=new_token,human=None,expected=403)
  if '--block4' in sys.argv:
   api('POST','/api/agent/commands',{'device_id':device,'command':'force_update'},expected=400)
   api('POST','/api/agent/commands',{'device_id':device,'command':'ping','payload':{'unexpected':True}},expected=400)
   queued=api('POST','/api/agent/commands',{'device_id':device,'command':'ping'},expected=201)['command_id']
   pending=api('GET','/api/agent/commands?device_id='+device,credential=new_token,human=None)['commands']
   assert pending[0]['id']==queued and 'expires_at' in pending[0]
   assert api('GET','/api/agent/commands?device_id='+device,credential=new_token,human=None)['commands']==[]
   sql("UPDATE agent_commands SET delivered_at=now()-interval '61 seconds' WHERE id='"+queued+"'")
   assert api('GET','/api/agent/commands?device_id='+device,credential=new_token,human=None)['commands'][0]['id']==queued
   ack={'status':'ok','result':{'status':'pong'}}
   api('POST','/api/agent/commands/'+queued+'/ack',ack,credential=new_token,human=None)
   api('POST','/api/agent/commands/'+queued+'/ack',ack,credential=new_token,human=None)
   api('POST','/api/agent/commands/'+queued+'/ack',{'status':'failed','result':{}},credential=new_token,human=None,expected=409)
   expired=api('POST','/api/agent/commands',{'device_id':device,'command':'status'},expected=201)['command_id']
   sql("UPDATE agent_commands SET created_at=now()-interval '2 hours' WHERE id='"+expired+"'")
   assert api('GET','/api/agent/commands?device_id='+device,credential=new_token,human=None)['commands']==[]
   api('POST','/api/agent/commands/'+expired+'/ack',ack,credential=new_token,human=None,expected=409)
   assert sql("SELECT status FROM agent_commands WHERE id='"+expired+"'")=='expired'
   api('GET','/api/devices/'+device+'/sessions/export',human=humans['viewer'],expected=403)
   api('GET','/api/history?device_id='+device+'&date=invalid',expected=400)
   api('GET','/api/usb?date=invalid',expected=400)
   initial=api('GET','/api/administration/policy/global')
   policy=initial['policy'];paused={**policy,'monitoring_paused':True}
   api('PUT','/api/administration/policy/global',{'revision':initial['revision'],'policy':paused,'reason':'Synthetic pause'})
   assert api('GET','/api/agent/policy?device_id='+device,credential=new_token,human=None)['policy']['monitoring_paused'] is True
   api('PUT','/api/administration/policy/'+device,{'revision':0,'policy':policy,'reason':'Synthetic override'})
   assert api('GET','/api/agent/policy?device_id='+device,credential=new_token,human=None)['policy']['monitoring_paused'] is False
   api('PUT','/api/administration/policy/'+device,{'revision':0,'policy':None,'reason':'Stale revision'},expected=409)
   api('PUT','/api/administration/policy/'+device,{'revision':1,'policy':None,'reason':'Restore inheritance'})
   assert api('GET','/api/agent/policy?device_id='+device,credential=new_token,human=None)['policy']['monitoring_paused'] is True
   invalid={**policy,'enabled_monitors':['heartbeat'],'disabled_monitors':['heartbeat']}
   api('PUT','/api/administration/policy/'+device,{'revision':2,'policy':invalid,'reason':'Conflict rejected'},expected=400)
   alert_body={'device_id':device,'alert_type':'synthetic','description':'Synthetic recurrence','severity':'LOW'}
   first=api('POST','/api/security/alerts',alert_body)['alert']['id']
   api('PATCH','/api/security/alerts/'+str(first)+'/resolve',{'resolution_notes':'first resolution'},human=humans['operator'])
   api('PATCH','/api/security/alerts/'+str(first)+'/resolve',{'resolution_notes':'do not overwrite'},human=humans['operator'],expected=404)
   second=api('POST','/api/security/alerts',alert_body)['alert']['id']
   assert first!=second and sql('SELECT resolved FROM security_alerts WHERE id='+str(first))=='t'
   assert sql('SELECT resolution_notes FROM security_alerts WHERE id='+str(first))=='first resolution'
   wait_for(lambda:api('GET','/ready',human=None))
   seq=0
   def publish_fixture(route,kind,payload,age=0):
    global seq
    seq+=1
    event={'event_id':str(uuid.uuid4()),'event_type':kind,'schema_version':2 if kind=='inventory_snapshot' else 1,'source_role':'service','boot_id':str(uuid.uuid4()),'sequence':seq,'device_id':device,'hostname':'SYNTHETIC','timestamp':(datetime.datetime.now(datetime.timezone.utc)-datetime.timedelta(seconds=age)).isoformat(),'payload':payload}
    fixture=path/'events.json';fixture.write_text(json.dumps([{'route':route,'event':event}]*2))
    run([str(PROBE)],env={**env,'RABBITMQ_URL':identity['rabbitmq_url'],'PROBE_DEVICE_ID':device,'PROBE_EVENTS_FILE':str(fixture)},timeout=30)
    wait_for(lambda:sql("SELECT EXISTS(SELECT 1 FROM processed_events WHERE event_id='"+event['event_id']+"')")=='t')
   inventory={'scope':'linux-dpkg','interval_days':1,'complete':True,'apps':[{'source_key':'synthetic','app_name':'Synthetic app','version':'1'}]}
   publish_fixture('inventory','inventory_snapshot',inventory,10)
   assert sql("SELECT count(*) FROM inventory_current WHERE device_id='"+device+"'")=='1'
   publish_fixture('inventory','inventory_snapshot',{**inventory,'apps':[]},20)
   publish_fixture('inventory','inventory_snapshot',{**inventory,'complete':False,'apps':[],'error':'Synthetic partial scan'},5)
   assert sql("SELECT count(*) FROM inventory_current WHERE device_id='"+device+"'")=='1'
   publish_fixture('inventory','inventory_snapshot',{**inventory,'apps':[]})
   assert sql("SELECT count(*) FROM inventory_current WHERE device_id='"+device+"'")=='0'
   assert sql("SELECT count(*) FROM inventory_changes WHERE device_id='"+device+"'")=='2'
   publish_fixture('usb','usb',{'action':'IN','device_name':'Synthetic USB','hardware_id':'SYNTHETIC'},20)
   publish_fixture('wifi','wifi',{'interface_name':'Synthetic','state':'connected','ssid':'SYNTHETIC'})
   assert sql("SELECT count(*) FROM usb_events WHERE device_id='"+device+"' AND timestamp<now()-interval '15 seconds'")=='1'
   assert sql("SELECT count(*) FROM wifi_events WHERE device_id='"+device+"'")=='1'
   print('PASS: real consumer inventory reconcile/partial/out-of-order and USB/WiFi replay deduplication',flush=True)
   print('PASS: policy inheritance, revision conflicts, pause/override and recurring alert evidence',flush=True)
   print('PASS: explicit command scope, leased redelivery, immutable/repeat ACK, expiry and query contracts',flush=True)
   raise SystemExit(0)
  probe_env=env.copy();probe_env.update(RABBITMQ_URL=identity['rabbitmq_url'],PROBE_DEVICE_ID=device,PROBE_SECONDS='1',PROBE_CACHE=str(path/'probe.db'))
  wait_for(lambda:api('GET','/ready',human=None))
  print('Publishing with native agent transport',flush=True)
  run([str(PROBE)],env=probe_env,timeout=40)
  wait_for(lambda:int(sql("SELECT count(*) FROM input_activity_metrics WHERE device_id='"+device+"'"))>0)
  run([str(PROBE)],env={**probe_env,'PROBE_DEVICE_ID':other,'PROBE_EXPECT_REJECTION':'true'},timeout=25)
  for operation in ['consume','configure']:run([str(PROBE)],env={**probe_env,'PROBE_FORBIDDEN_OPERATION':operation},timeout=25)
  run([str(PROBE)],env={**probe_env,'PROBE_SPOOF_PAYLOAD_UUID':other},timeout=25)
  wait_for(lambda:broker_api('queues/%2F/heartbeat_queue.dead').get('messages',0)>0)
  assert int(sql("SELECT count(*) FROM input_activity_metrics WHERE device_id='"+other+"'"))==0
  # Expiry window is moved only in this synthetic DB to exercise renewal without waiting 23 days.
  sql("UPDATE agent_credentials SET expires_at=now()+interval '2 days' WHERE credential_id='"+identity['credential_id']+"'")
  replacement=secrets.token_hex(32)
  renewed=api('POST','/api/agent/renew',{'new_token':replacement},credential=new_token,human=None)
  assert api('POST','/api/agent/renew',{'new_token':replacement},credential=new_token,human=None)['credential_id']==renewed['credential_id']
  held=subprocess.Popen([str(PROBE)],env={**probe_env,'PROBE_HOLD_FOR_REVOCATION':'true'},stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   wait_for(lambda:any(c['user']=='agent_'+identity['credential_id'] for c in broker_api('connections')))
   api('DELETE','/api/agent-credentials/'+device+'/'+identity['credential_id'],expected=204)
   held.communicate(timeout=30);assert held.returncode==0
  finally:
   if held.poll() is None:held.kill();held.wait()
  api('GET','/api/agent/policy?device_id='+device,credential=new_token,human=None,expected=401)
  run([str(PROBE)],env={**probe_env,'PROBE_EXPECT_REJECTION':'true'},timeout=25)
  run([str(PROBE)],env={**probe_env,'RABBITMQ_URL':renewed['rabbitmq_url'],'PROBE_CACHE':str(path/'renewed.db')},timeout=40)
  assert not any(u['name']=='agent_'+identity['credential_id'] for u in broker_api('users'))
  # Untrusted certificate must fail; no insecure trust fallback.
  bad_env={**probe_env,'RABBITMQ_URL':renewed['rabbitmq_url'],'SSL_CERT_FILE':str(path/'missing-ca.crt'),'SSL_CERT_DIR':str(path/'missing-ca-dir'),'PROBE_EXPECT_REJECTION':'true'}
  run([str(PROBE)],env=bad_env,timeout=25)
  # Run the actual agent credential module behind a local TLS proxy, without monitors/services.
  drops={'enroll':False,'renew':False}
  class Proxy(http.server.BaseHTTPRequestHandler):
   def log_message(self,*args):pass
   def do_POST(self):
    body=self.rfile.read(int(self.headers.get('Content-Length','0')))
    headers={key:self.headers[key] for key in ['Content-Type','x-agent-token'] if key in self.headers}
    request=urllib.request.Request(f'http://127.0.0.1:{httpport}'+self.path,data=body,headers=headers,method='POST')
    try:response=urllib.request.urlopen(request,timeout=70)
    except urllib.error.HTTPError as error:response=error
    with response:status=response.status;data=response.read()
    operation=self.path.rsplit('/',1)[-1]
    if drops.get(operation) and status==200:
     drops[operation]=False;self.connection.shutdown(socket.SHUT_RDWR);self.connection.close();return
    self.send_response(status);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
  proxy=http.server.ThreadingHTTPServer(('127.0.0.1',0),Proxy)
  tls=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);tls.load_cert_chain(str(path/'broker.crt'),str(path/'broker.key'))
  proxy.socket=tls.wrap_socket(proxy.socket,server_side=True)
  threading.Thread(target=proxy.serve_forever,daemon=True).start()
  managed=str(uuid.uuid4())
  api('POST','/api/devices/register',{'device_id':managed,'hostname':'SYNTHETIC_NATIVE_IDENTITY'},expected=200)
  invite=api('POST','/api/agent-credentials/enrollment/'+managed,{},expected=201)
  (path/'enrollment.code').write_text(invite['code'])
  data_dir=path/'agent-data';data_dir.mkdir();(data_dir/'pending-cache-sentinel').write_bytes(b'preserve-pending-events')
  cache_key=secrets.token_hex(32)
  managed_env={**probe_env,'PROBE_DEVICE_ID':managed,'PROBE_MANAGED_IDENTITY':'true','PROBE_ENROLL_ONLY':'true','AGENT_DATA_DIR':str(data_dir),'AGENT_OFFLINE_CACHE_KEY':cache_key,'AGENT_SERVER_URL':f'https://127.0.0.1:{proxy.server_address[1]}','AGENT_ENROLLMENT_CODE_FILE':str(path/'enrollment.code')}
  drops['enroll']=True
  first=subprocess.run([str(PROBE)],env=managed_env,capture_output=True,text=True,timeout=40)
  assert first.returncode!=0
  assert int(sql("SELECT count(*) FROM agent_credentials WHERE device_id='"+managed+"'"))==1
  run([str(PROBE)],env=managed_env,timeout=40)
  from cryptography.hazmat.primitives.ciphers.aead import AESGCM
  cipher=AESGCM(hashlib.sha256(cache_key.encode()).digest())
  def snapshot():
   sealed=json.loads((data_dir/'network-identity.json').read_text())
   return json.loads(cipher.decrypt(bytes(sealed['nonce']),bytes(sealed['ciphertext']),None))
  before=snapshot();assert before['device_id']==managed and before['pending_token'] is None
  assert before['token'] not in (data_dir/'network-identity.json').read_text()
  sql("UPDATE agent_credentials SET expires_at=now()+interval '2 days' WHERE device_id='"+managed+"'")
  before['expires_at']=(datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(days=2)).isoformat()
  nonce=secrets.token_bytes(12);sealed=cipher.encrypt(nonce,json.dumps(before).encode(),None)
  (data_dir/'network-identity.json').write_text(json.dumps({'nonce':list(nonce),'ciphertext':list(sealed)}))
  drops['renew']=True;run([str(PROBE)],env=managed_env,timeout=40)
  assert snapshot()['pending_token'] is not None
  run([str(PROBE)],env=managed_env,timeout=40)
  after=snapshot();assert after['token']!=before['token'] and after['pending_token'] is None
  assert int(sql("SELECT count(*) FROM agent_credentials WHERE device_id='"+managed+"'"))==2
  for row in api('GET','/api/agent-credentials/'+managed)['credentials']:api('DELETE','/api/agent-credentials/'+managed+'/'+row['credential_id'],expected=204)
  recovered=api('POST','/api/agent-credentials/enrollment/'+managed,{},expected=201)
  (path/'enrollment.code').write_text(recovered['code']);run([str(PROBE)],env=managed_env,timeout=40)
  assert snapshot()['device_id']==managed and snapshot()['token']!=after['token']
  assert (data_dir/'pending-cache-sentinel').read_bytes()==b'preserve-pending-events'
  print('PASS: native encrypted enrollment/renewal survive lost responses and restart; recovery preserves UUID and pending data')
  print('PASS: isolated production-mode migrations, TLS broker, enrollment/retry, admin scope, foreign UUID rejection, native publisher, renewal/retry, revocation and untrusted CA rejection')
 finally:
  if proxy is not None:proxy.shutdown();proxy.server_close()
  if process is not None:
   process.terminate()
   try:process.wait(timeout=10)
   except subprocess.TimeoutExpired:process.kill();process.wait()
  if 'log' in locals():log.close()
  for name in [BROKER,PG]:subprocess.run(['docker','rm','-f',name],capture_output=True)
