"""Check rootless HTTPS config using disposable certificates. Requires cryptography."""
import datetime, ipaddress, secrets, ssl, subprocess, tempfile, time, urllib.request
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.x509.oid import NameOID
ROOT=Path(__file__).resolve().parents[1]
NAME='monitor-nginx-check-'+secrets.token_hex(4)
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
 try:
  docker('run','--rm','-d','--name',NAME,'--user','65532:65532','--read-only','--cap-drop','ALL','--security-opt','no-new-privileges:true','--tmpfs','/tmp','-p','127.0.0.1::8443','-v',str(ROOT/'dashboard/nginx.production.conf')+':/etc/nginx/check.conf:ro','-v',str(path)+':/run/tls:ro','-v',str(ROOT/'dashboard/dist')+':/usr/share/nginx/html:ro','--entrypoint','nginx','monitorv32-local-dashboard:latest','-c','/etc/nginx/check.conf','-g','daemon off;')
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
  print('PASS: Nginx config, UID 65532/read-only/cap-drop, verified HTTPS and security headers; disposable certificate only')
 finally:
  subprocess.run(['docker','rm','-f',NAME],capture_output=True)
