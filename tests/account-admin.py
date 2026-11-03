"""Linux helper lifecycle; password hasher is stubbed, real hashing is covered by identity-stack."""
import importlib.util,json,os,stat,sys,tempfile
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('accounts','/test/manage-accounts.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as directory:
 root=Path(directory);root.chmod(0o700);users=root/'users.json';provision=root/'enrollment.txt'
 class HashResult:
  returncode=0
  stdout='$argon2id$v=19$m=19456,t=2,p=1$c3ludGhldGlj$synthetic'
 def action(*args):
  with patch.object(sys,'argv',['manage-accounts','--users-file',str(users),'--server-bin','/stub',*args]),patch.object(module.getpass,'getpass',return_value='synthetic-password-123'),patch.object(module.subprocess,'run',return_value=HashResult()):module.main()
 action('create','emergency','--emergency','--provisioning-file',str(provision))
 data=json.loads(users.read_text());assert data[0]['disabled'] and data[0]['role']=='admin' and data[0]['session_epoch']==1
 assert provision.read_text().startswith('otpauth://totp/') and stat.S_IMODE(provision.stat().st_mode)==0o600
 first_secret=data[0]['totp_secret']
 action('enable','emergency');assert json.loads(users.read_text())[0]['session_epoch']==2
 action('role','emergency','--role','viewer');assert json.loads(users.read_text())[0]['role']=='viewer'
 action('reset-password','emergency')
 action('reset-mfa','emergency','--provisioning-file',str(root/'replacement.txt'))
 data=json.loads(users.read_text());assert data[0]['totp_secret']!=first_secret and data[0]['session_epoch']==5
 action('disable','emergency');assert json.loads(users.read_text())[0]['disabled']
 log=Path(str(users)+'.audit.jsonl').read_text();assert len(log.splitlines())==12
 assert first_secret not in log and data[0]['totp_secret'] not in log and 'synthetic-password' not in log
 assert stat.S_IMODE(Path(str(users)+'.audit.jsonl').stat().st_mode)==0o600
 assert not list(root.glob('.accounts-*'))
print('PASS: Linux atomic account lifecycle, MFA rotation, session generation and private secret/audit files (hasher stubbed)')
