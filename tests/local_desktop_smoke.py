"""Read-only checks of native session desktop visibility in the isolated pilot."""
import json,datetime,urllib.request,urllib.parse,subprocess
from pathlib import Path
from http_auth import login_headers
root=Path(__file__).resolve().parents[1]
env=dict(line.split('=',1) for line in (root/'.local-test/.env').read_text().splitlines() if line and not line.startswith('#'))
base='http://127.0.0.1:18080/api'
auth={}
def get(path,body=None):
 headers={'Content-Type':'application/json'}
 headers.update(auth)
 req=urllib.request.Request(base+path,data=json.dumps(body).encode() if body else None,headers=headers)
 with urllib.request.urlopen(req,timeout=15) as response:return json.load(response)
assert subprocess.check_output(['docker','inspect','--format','{{index .Config.Labels "com.docker.compose.project"}}','monitorv32-local-server-1'],text=True).strip()=='monitorv32-local'
auth=login_headers(base.removesuffix('/api'),env['ADMIN_USERNAME'],env['ADMIN_PASSWORD'])
devices=get('/devices')['devices']
native=[d for d in devices if d.get('hostname')!='VALIDACION_SINTETICA_LOCAL']
assert native,'Install and start the native agent first'
results=[]
for d in native:
 device=d['device_id'];day=datetime.datetime.now(datetime.timezone.utc).date().isoformat()
 params=urllib.parse.urlencode({'device_id':device,'date':day,'tz_offset_minutes':0})
 history=get('/history?'+params);hourly=get('/history_hourly_programs?'+params)
 apps=get('/inventory/running_apps/'+device)['apps']
 live=[r for r in get('/live_devices')['devices'] if r['device_id']==device]
 assert history['success'] and history['history'],'Native window history missing'
 assert hourly['success'] and hourly['groups'],'Native hourly window history missing'
 assert apps,'Native open-app snapshot missing'
 assert live and live[0]['app_name']!='Sin actividad','Native recent activity missing'
 results.append({'device_id':device,'history_groups':len(history['history']),'hourly_groups':len(hourly['groups']),'open_apps':len(apps),'recent_activity':True})
report={'result':'passed','at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'native_desktop':results}
(root/'.local-test/logs/desktop-smoke.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report,indent=2))
