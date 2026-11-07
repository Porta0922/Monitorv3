#!/usr/bin/env python3
"""Check or renew leaf certificates under the existing CA on the Linux candidate host."""
import argparse, json, os, shutil, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]

def check(directory,web,broker,seconds):
 def openssl(*args):
  result=subprocess.run(['openssl',*args],capture_output=True)
  if result.returncode:raise RuntimeError('Certificate validation failed: chain, hostname, lifetime or key')
  return result.stdout.strip()
 for prefix,host in [('web',web),('broker',broker)]:
  cert,key=directory/(prefix+'.crt'),directory/(prefix+'.key')
  openssl('x509','-in',str(cert),'-checkend',str(seconds),'-noout')
  openssl('verify','-CAfile',str(directory/'ca.crt'),'-untrusted',str(cert),'-verify_hostname',host,str(cert))
  if openssl('x509','-in',str(cert),'-pubkey','-noout')!=openssl('pkey','-in',str(key),'-passin','pass:','-pubout'):raise RuntimeError('Private key mismatch')
 openssl('verify','-CAfile',str(directory/'ca.crt'),'-untrusted',str(directory/'broker.crt'),'-verify_hostname','rabbitmq',str(directory/'broker.crt'))

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--install-from',type=Path);p.add_argument('--warning-days',type=int,default=14);p.add_argument('--rollback-pending',action='store_true')
 p.add_argument('--project-name',default='monitor-production-candidate')
 p.add_argument('--env-file',type=Path,help='Same interpolation file used for deployment')
 p.add_argument('--compose-file',action='append',type=Path,default=[],help='Additional overlays, in deployment order')
 a=p.parse_args()
 if os.name!='posix':p.error('Run on the Linux candidate host')
 if not 1<=a.warning_days<=90:p.error('warning-days must be 1..90')
 command=['docker','compose','--project-name',a.project_name]
 if a.env_file:
  if not a.env_file.is_file():p.error('Environment file missing')
  command+=['--env-file',str(a.env_file.resolve())]
 command+=['-f','docker-compose.yml','-f','docker-compose.production.yml']
 for overlay in a.compose_file:
  if not overlay.is_file():p.error('Compose overlay missing')
  command+=['-f',str(overlay.resolve())]
 private=ROOT/'.production';current=private/'tls';meta=json.loads((private/'deployment.json').read_text())
 backup=private/'tls-renewal-pending'
 if a.rollback_pending:
  if a.install_from:p.error('Choose install or rollback')
  if not backup.is_dir():p.error('No pending renewal backup')
  check(backup,meta['web_host'],meta['broker_host'],0)
  for name in ['web.crt','web.key','broker.crt','broker.key']:
   staged=current/(name+'.next');shutil.copyfile(backup/name,staged);staged.chmod(0o444);os.replace(staged,current/name)
  subprocess.run(command+['up','-d','--no-deps','--force-recreate','--wait','--wait-timeout','180','rabbitmq','server','dashboard'],cwd=ROOT,check=True)
  for name in ['web.crt','web.key','broker.crt','broker.key','ca.crt']:(backup/name).unlink()
  backup.rmdir();print('Previous leaf certificates restored');return
 if backup.exists():p.error('Interrupted renewal exists; review and use --rollback-pending before another renewal')
 if not a.install_from:
  check(current,meta['web_host'],meta['broker_host'],a.warning_days*86400);print('Certificate checks passed; no changes made');return
 source=a.install_from.resolve();check(source,meta['web_host'],meta['broker_host'],86400)
 if (source/'ca.crt').read_bytes()!=(current/'ca.crt').read_bytes():p.error('CA rotation needs staged trust distribution; only leaf renewal is supported here')
 os.umask(0o077)
 subprocess.run(command+['config','--quiet'],cwd=ROOT,check=True)
 # Durable backup also acts as an exclusive operation marker. Never overwrite it.
 backup=private/'tls-renewal-pending';backup.mkdir(mode=0o700)
 names=['web.crt','web.key','broker.crt','broker.key']
 for name in names+['ca.crt']:shutil.copyfile(current/name,backup/name)
 def replace(directory):
  for name in names:
   staged=current/(name+'.next');shutil.copyfile(directory/name,staged);staged.chmod(0o444);os.replace(staged,current/name)
 def restart():subprocess.run(command+['up','-d','--no-deps','--force-recreate','--wait','--wait-timeout','180','rabbitmq','server','dashboard'],cwd=ROOT,check=True)
 try:replace(source);restart()
 except BaseException:
  # Keep the durable backup for operator inspection even after successful rollback.
  replace(backup);restart();raise
 for name in names+['ca.crt']:(backup/name).unlink()
 backup.rmdir()
 print('Leaf certificates renewed; services recreated. Verify agent reconnects. CA unchanged.')
if __name__=='__main__':main()
