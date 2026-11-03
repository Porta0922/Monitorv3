"""Executed inside identity-stack.py's disposable production-mode fixture."""
print('Checking human session/MFA/role/CSRF matrix',flush=True)
for prefix in ['', '/api']:
 for role in ['viewer','operator']:
  human=humans[role]
  api('GET',prefix+'/devices',human=human)
  for route in ['/administration/accounts','/maintenance/status','/audit','/auth/security']:
   api('GET',prefix+route,human=human,expected=403)
  for method,route in [('PATCH','/devices/'+str(uuid.uuid4())),('PUT','/administration/policy/global'),('POST','/agent/commands'),('POST','/maintenance/dead-letters/replay')]:
   api(method,prefix+route,{},human=human,expected=403)
  if role=='viewer':
   api('GET',prefix+'/export/csv',human=human,expected=403)
   api('POST',prefix+'/security/investigations',{},human=human,expected=403)
 api('GET',prefix+'/auth/session',human=token)
api('POST','/auth/logout',{},human={'Cookie':token['Cookie']},expected=403)
api('POST','/auth/logout',{},human={**token,'Origin':'https://attacker.invalid'},expected=403)
api('GET','/auth/session',human={'Cookie':'__Host-monitor_session='+'0'*64},expected=401)
api('GET','/auth/session',human={'Cookie':token['Cookie']+'; '+token['Cookie']},expected=401)
security=api('GET','/auth/security');assert security['failed_logins_15m']>=2
for _ in range(5):login('unknown-account',expected=401)
login('unknown-account',expected=429)
assert api('GET','/auth/security')['limited_accounts_24h']>=1
# Account files are reread, malformed configuration fails closed, no fallback bootstrap.
original=(path/'users.json').read_text()
(path/'users.json').write_text('{broken')
api('GET','/auth/session',human=humans['viewer'],expected=503)
(path/'users.json').write_text(original)
viewer=next(a for a in accounts if a['username']=='viewer')
viewer['disabled']=True;(path/'users.json').write_text(json.dumps(accounts))
api('GET','/auth/session',human=humans['viewer'],expected=401)
viewer['disabled']=False;viewer['session_epoch']=1;(path/'users.json').write_text(json.dumps(accounts))
api('GET','/auth/session',human=humans['viewer'],expected=401)
humans['viewer']=login('viewer')
operator=next(a for a in accounts if a['username']=='operator')
operator['role']='viewer';(path/'users.json').write_text(json.dumps(accounts))
assert api('GET','/auth/session',human=humans['operator'])['role']=='viewer'
operator['role']='operator';(path/'users.json').write_text(json.dumps(accounts))
assert api('GET','/auth/session',human=humans['operator'])['role']=='viewer'
humans['operator']=login('operator')
# Own session management cannot revoke a different user's session.
other_id=api('GET','/auth/sessions',human=humans['operator'])['sessions'][0]['session_id']
api('DELETE','/auth/sessions/'+other_id,human=humans['viewer'],expected=404)
api('DELETE','/auth/users/operator/sessions',human=humans['viewer'],expected=403)
api('DELETE','/auth/users/operator/sessions',expected=204)
api('GET','/auth/session',human=humans['operator'],expected=401)
humans['operator']=login('operator')
api('POST','/auth/logout',{},human=humans['operator'])
api('GET','/auth/session',human=humans['operator'],expected=401)
humans['operator']=login('operator')
api('POST','/auth/revoke-all',{},human=humans['operator'],expected=204)
api('GET','/auth/session',human=humans['operator'],expected=401)
humans['operator']=login('operator')
sql("UPDATE human_sessions SET last_seen=now()-interval '31 minutes' WHERE username='viewer'")
api('GET','/auth/session',human=humans['viewer'],expected=401)
humans['viewer']=login('viewer')
sql("UPDATE human_sessions SET expires_at=now()-interval '1 second' WHERE username='viewer'")
api('GET','/auth/session',human=humans['viewer'],expected=401)
humans['viewer']=login('viewer')
assert int(sql("SELECT count(*) FROM audit_events WHERE action='auth.sessions.revoked'"))>=3
assert not any(value in json.dumps(api('GET','/administration/accounts')) for value in [password,password_hash,mfa_secret])
print('PASS: production cookies, MFA replay, CSRF, both route prefixes, role changes, invalid config, expiry, revocation and safe account projection',flush=True)

audit_device=str(uuid.uuid4())
api('POST','/api/devices/register',{'device_id':audit_device,'hostname':'SYNTHETIC_AUTH'})
api('PATCH','/api/devices/'+audit_device,{'nickname':'Auth validation'})
for prefix in ['', '/api']:
 exported=api('GET',prefix+'/export/csv',human=humans['operator']);assert exported.startswith('device_id,')
 investigation=api('POST',prefix+'/security/investigations',{'device_id':audit_device,'title':'Synthetic','reason':'sensitive reason excluded from generic audit','event_from':'2026-01-01T00:00:00Z','event_to':'2026-01-02T00:00:00Z'},human=humans['operator'],expected=201)
 api('POST',prefix+'/security/investigations/'+investigation['id']+'/release',{'reason':'Synthetic release'},human=humans['operator'])
assert int(sql("SELECT count(*) FROM audit_events WHERE target LIKE '%"+audit_device+"%' AND details::jsonb->>'result'='status_201'"))>=2
assert sql("SELECT count(*) FROM audit_events WHERE details LIKE '%sensitive reason%'")=='0'
print('PASS: operator export/investigation execution and actor/device/result audit with private request content excluded',flush=True)
