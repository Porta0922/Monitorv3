"""Read-only credential-pattern scan. Never prints secret values or tests credentials.
Scans current Git-visible files, local env metadata and reachable historical blobs.
Evidence is private; does not certify absence of unknown token formats.
"""
import hashlib,json,re,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT)
PATTERNS={
 'github_token':re.compile(rb'(?:github_pat_[A-Za-z0-9_]{50,}|gh[pousr]_[A-Za-z0-9]{30,})'),
 'private_key':re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----'),
 'aws_access_key':re.compile(rb'(?:AKIA|ASIA)[A-Z0-9]{16}'),
 'credential_url':re.compile(rb'(?:amqps?|postgres(?:ql)?|redis)://[^\s:/]+:([^\s@]+)@'),
}
# Compare actual pilot secrets in memory; never write them to the report.
for env_name in ['.local-test/.env','.local-test/agent.env']:
 env_path=ROOT/env_name
 if env_path.is_file():
  for line in env_path.read_text(encoding='utf-8-sig').splitlines():
   if not line or line.startswith('#') or '=' not in line:continue
   key,value=line.split('=',1)
   if len(value)>=16 and any(word in key.upper() for word in ['PASSWORD','TOKEN','SECRET','KEY']):
    PATTERNS['local_secret:'+key]=re.compile(re.escape(value.encode()))
def findings(data):
 result=[]
 for kind,pattern in PATTERNS.items():
  for match in pattern.finditer(data):
   secret=match.group(1) if kind=='credential_url' else match.group()
   if kind=='credential_url' and (len(secret)<16 or any(x in secret.lower() for x in [b'password',b'change',b'example',b'${',b'{',b'%s'])):continue
   result.append(dict(synthetic_candidate=len(set(secret))<8,kind=kind,fingerprint=hashlib.sha256(secret).hexdigest()[:16]))
 return result
paths=set(git('ls-files','-z').decode().split('\0'))|set(git('ls-files','--others','--exclude-standard','-z').decode().split('\0'))
current=[]
for name in sorted(paths):
 p=ROOT/name
 if name and p.is_file() and p.stat().st_size<4*1024*1024:
  current += [dict(path=name,**f) for f in findings(p.read_bytes())]
objects={}
for line in git('rev-list','--objects','--all').decode().splitlines():
 parts=line.split(' ',1);objects[parts[0]]=parts[1] if len(parts)>1 else ''
metadata=git('cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype) %(objectsize)').decode().splitlines()
blobs=[(sha,int(size)) for sha,kind,size in (line.split() for line in metadata) if sha in objects and kind=='blob' and int(size)<=4*1024*1024]
proc=subprocess.Popen(['git','cat-file','--batch'],cwd=ROOT,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
historical=[]
for sha,size in blobs:
 proc.stdin.write((sha+'\n').encode());proc.stdin.flush();header=proc.stdout.readline();data=proc.stdout.read(size);proc.stdout.read(1)
 historical += [dict(blob=sha,path=objects[sha],**f) for f in findings(data)]
proc.stdin.close();proc.wait()
envs=[]
for name in ['.env','.env.example','.local-test/.env','.local-test/agent.env','.production/endpoint.env']:
 p=ROOT/name
 if not p.is_file():continue
 keys=[]
 for line in p.read_text(encoding='utf-8-sig').splitlines():
  if not line.strip() or line.lstrip().startswith('#') or '=' not in line:continue
  key,value=line.split('=',1);key=key.strip();value=value.strip().strip('"')
  sensitive=any(word in key.upper() for word in ['PASSWORD','TOKEN','SECRET','KEY'])
  keys.append(dict(name=key,empty=not value,sensitive=sensitive,placeholder=any(x in value.lower() for x in ['change-me','change_password','example','<password','dev-secret']),debug_enabled=key in ['DEBUG','RUST_LOG'] and any(x in value.lower() for x in ['debug','trace','true'])))
 envs.append(dict(path=name,tracked=bool(git('ls-files','--',name).strip()),variables=keys))
report=dict(scope='reachable local refs; known patterns; blobs <=4 MiB; no validity probing',scanned_blobs=len(blobs),current=current,historical=historical,envs=envs)
output=ROOT/'.local-test/security';output.mkdir(parents=True,exist_ok=True)
(output/'inventory.json').write_text(json.dumps(report,indent=2))
print(json.dumps(dict(scanned_blobs=len(blobs),current_matches=len(current),historical_unique=len({(f['kind'],f['fingerprint']) for f in historical}),report='.local-test/security/inventory.json')))
