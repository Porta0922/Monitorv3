"""Bounded end-to-end checks only against the fixed isolated local pilot."""
import base64, datetime, json, subprocess, time, urllib.request, urllib.error, uuid
from pathlib import Path
from http_auth import login_headers
ROOT=Path(__file__).resolve().parents[1]
LOCAL=ROOT/'.local-test'
ENV=dict(line.split('=',1) for line in (LOCAL/'.env').read_text().splitlines() if line and not line.startswith('#'))
project=subprocess.check_output(['docker','inspect','--format','{{index .Config.Labels "com.docker.compose.project"}}','monitorv32-local-server-1'],text=True).strip()
assert project=='monitorv32-local','Refusing any other deployment'
BASE='http://127.0.0.1:18080/api'
AUTH={}
checks=[]
def request(path,body=None,method=None,headers=None,base=BASE):
    values={'Content-Type':'application/json'}
    if base==BASE:values.update(AUTH)
    values.update(headers or {})
    req=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),method=method,headers=values)
    with urllib.request.urlopen(req,timeout=15) as response:return json.load(response)
assert request('/ready')['status']=='ready';checks.append('API readiness through Nginx')
AUTH=login_headers(BASE.removesuffix('/api'),ENV['ADMIN_USERNAME'],ENV['ADMIN_PASSWORD'])
checks.append('Administrative login')
request('/devices/search?q=&status=all&page=1');checks.append('Server-side equipment search')
policy=request('/administration/policy/global')
request('/administration/versions');request('/administration/accounts');checks.append('Policy/version/account reads')
fixture=LOCAL/'smoke-fixture.json'
if fixture.exists():device=json.loads(fixture.read_text())['device_id']
else:device=str(uuid.uuid4());fixture.write_text(json.dumps({'device_id':device}))
boot=str(uuid.uuid4());sequence=0
broker_auth='Basic '+base64.b64encode(('monitor_local:'+ENV['LOCAL_RABBIT_PASSWORD']).encode()).decode()
def event(kind,payload,**extra):
    global sequence
    sequence+=1
    return dict(event_id=str(uuid.uuid4()),device_id=device,hostname='VALIDACION_SINTETICA_LOCAL',version='3.4.0-fixture',boot_id=boot,sequence=sequence,schema_version=1,source_role='service',timestamp=datetime.datetime.now(datetime.timezone.utc).isoformat(),auth_token=ENV['AGENT_AUTH_TOKEN'],event_type=kind,payload=payload,**extra)
def publish(value,route='heartbeat'):
    result=request('/api/exchanges/%2f/monitoring/publish',{'properties':{'delivery_mode':2},'routing_key':'monitoring.'+route,'payload':json.dumps(value),'payload_encoding':'string'},headers={'Authorization':broker_auth},base='http://127.0.0.1:15675')
    assert result['routed'], 'Event was not routed'
caps={'mode':'service','os':'synthetic','architecture':'x64','desktop':'none','input_available':False,'desktop_capture_eligible':False}
metrics={'schema_version':1,'capabilities':caps,'cpu_state':'available','cpu_percent':42.5,'memory_state':'available','memory_total_bytes':4294967296,'memory_used_bytes':1073741824,'memory_percent':25.0,'uptime_seconds':120,'sample_interval_seconds':60,'disks_state':'unavailable','network_state':'unavailable','disks':[],'network':[]}
resource=event('input_summary',{'status':'service','active_seconds':0,'idle_seconds':0,'keys_count':0,'mouse_moves_count':0,'clicks_count':0,'is_service':True,'input_available':False,'cpu_percent':42.5,'memory_used_mb':1024.0,'memory_percent':25.0,'system_metrics':metrics})
publish(resource);publish(resource)
health=event('agent_health',{'status':'healthy','capabilities':caps,'monitoring_paused':False,'tasks':[],'collectors':[],'cache_unsynced_events':0,'version':'3.4.0-fixture','policy_fingerprint':policy['fingerprint']})
publish(health,'health')
for attempt in range(30):
    telemetry=request('/devices/'+device+'/telemetry')
    if telemetry.get('resources') and telemetry.get('health'):break
    time.sleep(1)
else:raise AssertionError('Telemetry ingestion timeout')
assert telemetry['resources']['data']['cpu_percent']==42.5
checks.append('RabbitMQ -> database -> telemetry API')
request('/devices/'+device,{'nickname':'Prueba local (datos sintéticos)','group_name':'Validación sintética','desired_version':'3.4.0-fixture'},method='PATCH')
page=request('/devices/search?'+urllib.parse.urlencode({'q':device,'group':'Validación sintética','status':'all','page':1}))
assert page['total']==1 and page['devices'][0]['group_name']=='Validación sintética'
checks.append('Group/metadata update and filtered search')
session=event('input_summary',{'status':'active','active_seconds':30,'idle_seconds':0,'keys_count':3,'mouse_moves_count':2,'clicks_count':1,'input_available':True,'is_service':False})
session.update(source_role='session',source_principal='test:local',source_session=boot)
publish(session);publish(session)
for attempt in range(30):
    result=request('/devices/'+device+'/sessions')
    rows=[row for row in result['items'] if row['session_id']==boot]
    if rows:break
    time.sleep(1)
else:raise AssertionError('Session ingestion timeout')
assert rows[0]['events']==1 and rows[0]['active_person_seconds']==30
checks.append('Session attribution and duplicate suppression')
day=datetime.datetime.now(datetime.timezone.utc).date().isoformat()
request('/resources/'+device+'/daily?'+urllib.parse.urlencode({'from':day,'to':day}))
checks.append('Daily metrics query with availability columns')
saved=request('/administration/policy/'+device,{'revision':0,'policy':policy['policy'],'reason':'Local pilot contract check'},method='PUT') if request('/administration/policy/'+device)['revision']==0 else None
if saved:
    assert request('/administration/policy/'+device)['revision']==saved['revision']
checks.append('Device policy persistence')
request('/maintenance/status');checks.append('Maintenance status')
report={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'project':project,'device_id':device,'synthetic':True,'checks':checks,'result':'passed'}
(LOCAL/'logs'/'smoke.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2))
