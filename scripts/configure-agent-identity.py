#!/usr/bin/env python3
"""Configure production service enrollment without changing UUID, cache keys or pending events."""
import argparse, ctypes, os, re, secrets, subprocess, tempfile
from pathlib import Path
from urllib.parse import urlparse

def protect(path):
 if os.name=='nt':
  result=subprocess.run(['icacls',str(path),'/inheritance:r','/grant:r','*S-1-5-18:F','*S-1-5-32-544:F'],capture_output=True)
  if result.returncode:raise RuntimeError('Cannot protect service configuration ACL')
 else:os.chmod(path,0o600)

def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--config',type=Path,required=True);p.add_argument('--data-dir',type=Path,required=True)
 p.add_argument('--server-url',required=True);p.add_argument('--code-file',type=Path,required=True);p.add_argument('--ca-file',type=Path,required=True)
 a=p.parse_args()
 if (os.name=='nt' and not ctypes.windll.shell32.IsUserAnAdmin()) or (os.name!='nt' and os.geteuid()!=0):p.error('Administrative privileges required for service configuration')
 url=urlparse(a.server_url)
 if url.scheme!='https' or not url.hostname or url.username or url.password or url.query or url.fragment:p.error('Use an HTTPS server URL without credentials/query')
 for path in [a.config,a.data_dir,a.code_file,a.ca_file]:
  if not path.is_absolute() or path.is_symlink():p.error('Paths must be absolute and not symbolic links')
 if not a.config.is_file():p.error('Configure the installed service first; existing config required')
 code=a.code_file.read_text().strip()
 if not re.fullmatch('[0-9a-fA-F]{64}',code):p.error('Invalid enrollment code')
 if not a.ca_file.is_file():p.error('CA file missing')
 original=a.config.read_text(encoding='utf-8-sig')
 def value(name):
  rows=re.findall(r'^'+re.escape(name)+r'=(.*)$',original,re.M)
  if len(rows)>1:p.error('Duplicate setting '+name)
  return rows[0].strip().strip('"') if rows else None
 key=value('AGENT_OFFLINE_CACHE_KEY')
 if not key:
  if list(a.data_dir.glob('*.db')):p.error('Existing cache without explicit key: migrate its key before enabling production')
  key=secrets.token_hex(32)
 if not re.fullmatch('[0-9a-fA-F]{64}',key) or len(set(key))<2:p.error('Existing cache key is incompatible; do not replace it without migrating pending data')
 changes={'MONITOR_ENV':'production','AGENT_OFFLINE_CACHE_KEY':key,'AGENT_SERVER_URL':a.server_url.rstrip('/'),'AGENT_ENROLLMENT_CODE_FILE':a.code_file.as_posix(),'SSL_CERT_FILE':a.ca_file.as_posix()}
 if value('AGENT_DATA_DIR') and Path(value('AGENT_DATA_DIR')).resolve()!=a.data_dir.resolve():p.error('data-dir differs from installed configuration; preserve device identity')
 changes['AGENT_DATA_DIR']=a.data_dir.as_posix()
 lines=[line for line in original.splitlines() if not any(line.startswith(k+'=') for k in [*changes,'AGENT_AUTH_TOKEN','RABBITMQ_URL'])]
 lines.extend(k+'="'+v.replace('"','\\"')+'"' for k,v in changes.items())
 protect(a.code_file);protect(a.config)
 fd,name=tempfile.mkstemp(prefix='.identity-',dir=a.config.parent)
 try:
  os.close(fd);protect(Path(name));Path(name).write_text('\n'.join(lines)+'\n',encoding='utf-8');os.replace(name,a.config);protect(a.config)
 finally:
  if Path(name).exists():Path(name).unlink()
 print('Service configuration prepared. UUID/cache retained. Restart the installed service using its lifecycle script; no restart was performed.')
if __name__=='__main__':main()
