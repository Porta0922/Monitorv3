"""Exercise the built production image without pilot credentials or host ports."""
import argparse
import io
import json
import subprocess
import tarfile
import time
import uuid


def docker(*args, **kwargs):
    return subprocess.run(['docker', *args], capture_output=True, timeout=120, **kwargs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--image', required=True)
    parser.add_argument('--network', choices=['monitor-refactor-test_activity-monitor'])
    args = parser.parse_args()
    image = args.image
    info = json.loads(docker('image', 'inspect', image, text=True, check=True).stdout)[0]
    assert info['Config']['User'] == '65532:65532'
    name = 'monitor-runtime-check-' + uuid.uuid4().hex[:10]
    backend = name + '-http'
    application = name + '-app'
    try:
        docker('create', '--name', name, image, check=True)
        exported = docker('export', name, check=True).stdout
        with tarfile.open(fileobj=io.BytesIO(exported)) as archive:
            paths = set(archive.getnames())
        for path in ['bin/sh', 'bin/bash', 'usr/bin/sh', 'usr/bin/bash', 'usr/bin/curl', 'usr/bin/apt-get']:
            assert path not in paths, path
        result = docker('run', '--rm', '-i', '--network', 'none', '--read-only', '--cap-drop', 'ALL',
                        '--security-opt', 'no-new-privileges:true', image,
                        '/usr/local/bin/activity-monitor-server', '--hash-password-stdin',
                        input=b'disposable-runtime-fixture-password')
        assert result.returncode == 0 and result.stdout.startswith(b'$argon2id$'), 'Password helper failed'
        script = '''import http.server,os
class Handler(http.server.BaseHTTPRequestHandler):
 def do_GET(self):
  status=int(open('/tmp/status').read()); self.send_response(status); self.end_headers()
 def log_message(self,*args): pass
open('/tmp/status','w').write('200')
http.server.HTTPServer(('0.0.0.0',3000),Handler).serve_forever()
'''
        docker('run', '-d', '--name', backend, '--network', 'none', 'python:3.12-alpine', 'python', '-c', script, check=True)
        for attempt in range(30):
            ready = docker('exec', backend, 'python', '-c', "import urllib.request; urllib.request.urlopen('http://127.0.0.1:3000/ready',timeout=1)")
            if ready.returncode == 0: break
            time.sleep(1)
        else: raise AssertionError('Disposable HTTP fixture did not start: ' + docker('logs', backend, text=True).stderr[-2000:])
        for status in [200, 503, 302]:
            docker('exec', backend, 'python', '-c', f"open('/tmp/status','w').write('{status}')", check=True)
            result = docker('run', '--rm', '--read-only', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges:true',
                            '--network', 'container:' + backend, '-e', 'HTTP_PROXY=http://127.0.0.1:1',
                            image, '/usr/local/bin/activity-monitor-server', '--healthcheck')
            assert (result.returncode == 0) == (status == 200), f'Health status {status} failed'
        if args.network:
            docker('run', '-d', '--name', application, '--read-only', '--tmpfs', '/tmp',
                   '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges:true',
                   '--network', args.network, '-e', 'MONITOR_ENV=development',
                   '-e', 'DATABASE_URL=postgresql://monitor_user:monitor_password@postgres:5432/activity_monitor',
                   '-e', 'RABBITMQ_URL=amqp://guest:guest@rabbitmq:5672/%2f',
                   '-e', 'JWT_SECRET=disposable-runtime-check', '-e', 'ADMIN_PASSWORD=disposable-runtime-check',
                   image, check=True)
            for attempt in range(60):
                ready = docker('exec', application, '/usr/local/bin/activity-monitor-server', '--healthcheck')
                if ready.returncode == 0: break
                time.sleep(1)
            else: raise AssertionError('Production runtime did not reach readiness with disposable DB/broker')
            print('PASS: production runtime starts with disposable PostgreSQL/RabbitMQ under read-only nonroot restrictions')
        print('PASS: production UID, shell/tool absence, offline Argon2, readiness success/failure/redirect and proxy isolation')
    finally:
        docker('rm', '-f', name, backend, application)


if __name__ == '__main__': main()
