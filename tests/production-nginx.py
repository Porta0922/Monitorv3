"""Check rootless HTTPS config using disposable certificates. Requires cryptography."""
import datetime, ipaddress, os, secrets, ssl, subprocess, tempfile, time, urllib.request, urllib.error
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID
ROOT=Path(__file__).resolve().parents[1]
NAME='monitor-nginx-check-'+secrets.token_hex(4)
IMAGE=os.environ.get('NGINX_TEST_IMAGE','monitorv32-local-dashboard:latest')
def docker(*args):
 r=subprocess.run(['docker',*args],capture_output=True,text=True)
 if r.returncode: raise RuntimeError(r.stderr[-2000:])
 return r.stdout.strip()
with tempfile.TemporaryDirectory(prefix='monitor-tls-check-') as directory:
 path=Path(directory)
 key=rsa.generate_private_key(public_exponent=65537,key_size=2048)
 subject=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'localhost')])
 now=datetime.datetime.now(datetime.timezone.utc)
 cert=(x509.CertificateBuilder().subject_name(subject).issuer_name(subject).public_key(key.public_key()).serial_number(x509.random_serial_number()).not_valid_before(now-datetime.timedelta(minutes=1)).not_valid_after(now+datetime.timedelta(days=1)).add_extension(x509.SubjectAlternativeName([x509.DNSName('localhost'),x509.IPAddress(ipaddress.ip_address('127.0.0.1'))]),critical=False).sign(key,hashes.SHA256()))
 (path/'web.crt').write_bytes(cert.public_bytes(serialization.Encoding.PEM))
 (path/'web.key').write_bytes(key.private_bytes(serialization.Encoding.PEM,serialization.PrivateFormat.PKCS8,serialization.NoEncryption()))
 (path/'stub.conf').write_text('events {} http { server { listen 3000; location / { return 200 "{}"; } } }')
 try:
  docker('network','create',NAME)
  docker('run','--rm','-d','--name',NAME+'-stub','--network',NAME,'--network-alias','server','-v',str(path/'stub.conf')+':/etc/nginx/stub.conf:ro','--entrypoint','nginx',IMAGE,'-c','/etc/nginx/stub.conf','-g','daemon off;')
  docker('run','--rm','-d','--name',NAME,'--network',NAME,'--user','65532:65532','--read-only','--cap-drop','ALL','--security-opt','no-new-privileges:true','--tmpfs','/tmp','-p','127.0.0.1::8443','-v',str(ROOT/'dashboard/nginx.production.conf')+':/etc/nginx/check.conf:ro','-v',str(path)+':/run/tls:ro','-v',str(ROOT/'dashboard/dist')+':/usr/share/nginx/html:ro','--entrypoint','nginx',IMAGE,'-c','/etc/nginx/check.conf','-g','daemon off;')
  docker('exec',NAME,'nginx','-c','/etc/nginx/check.conf','-t')
  port=docker('port',NAME,'8443/tcp').rsplit(':',1)[1]
  context=ssl.create_default_context(cafile=str(path/'web.crt'))
  for attempt in range(20):
   try:
    with urllib.request.urlopen('https://127.0.0.1:'+port+'/',context=context,timeout=2) as response:
     assert response.status==200
     assert response.headers['Strict-Transport-Security']=='max-age=31536000'
     assert response.headers['X-Frame-Options']=='DENY'
     assert response.headers['X-Content-Type-Options']=='nosniff'
     assert "script-src 'self'" in response.headers['Content-Security-Policy']
     assert "frame-ancestors 'none'" in response.headers['Content-Security-Policy']
     assert '<script>' not in response.read().decode()
    with urllib.request.urlopen('https://127.0.0.1:'+port+'/theme-init.js',context=context,timeout=2) as theme:
     assert 'monitor-theme' in theme.read().decode()
    break
   except OSError:
    time.sleep(.5)
  else: raise RuntimeError('HTTPS unavailable')
  base='https://127.0.0.1:'+port
  for route in ['/.env','/.git/config','/backup.sql','/assets/app.js.map','/private.key']:
   try:
    urllib.request.urlopen(base+route,context=context,timeout=3)
    raise AssertionError('Sensitive path served: '+route)
   except urllib.error.HTTPError as error:
    assert error.code==404, (route,error.code)
    assert error.headers['X-Content-Type-Options']=='nosniff'
    assert error.headers['X-Frame-Options']=='DENY'
  # Synthetic backend avoids touching real accounts or the running pilot.
  statuses=[]
  for _ in range(16):
   try:
    with urllib.request.urlopen(urllib.request.Request(base+'/api/auth/login',data=b'{}',headers={'Content-Type':'application/json'}),context=context,timeout=3) as response:
     statuses.append(response.status)
   except urllib.error.HTTPError as error:
    statuses.append(error.code)
  assert 429 in statuses, statuses
  assert 200 in statuses, statuses
  canary='PRIVATE_QUERY_CANARY_'+secrets.token_hex(8)
  urllib.request.urlopen(base+'/?probe='+canary,context=context,timeout=3).close()
  logs=docker('logs',NAME)
  assert canary not in logs, 'Query leaked into access logs'
  print('PASS: Nginx config, rootless/read-only, verified HTTPS, security headers, sensitive file denial, login rate limit and query-free access logs')
 finally:
  subprocess.run(['docker','rm','-f',NAME],capture_output=True)
  subprocess.run(['docker','rm','-f',NAME+'-stub'],capture_output=True)
  subprocess.run(['docker','network','rm',NAME],capture_output=True)
