"""Force an API IP change ONLY in the fixed disposable monitor-refactor-test stack."""
import json
import subprocess
import time
import urllib.request
import uuid

PROJECT = 'monitor-refactor-test'
NETWORK = PROJECT + '_activity-monitor'
SERVER = PROJECT + '-server-1'
DASHBOARD = PROJECT + '-dashboard-1'
COMPOSE = ['docker', 'compose', '-p', PROJECT, '-f', 'docker-compose.yml', '-f', 'tests/compose.smoke.yml']

def run(*args):
    return subprocess.check_output(args, text=True, timeout=180).strip()

def inspect(name):
    return json.loads(run('docker', 'inspect', name))[0]

def ready():
    with urllib.request.urlopen('http://127.0.0.1:15173/api/ready', timeout=5) as response:
        return json.load(response)['status'] == 'ready'

server = inspect(SERVER)
dashboard = inspect(DASHBOARD)
assert server['Config']['Labels']['com.docker.compose.project'] == PROJECT
assert dashboard['Config']['Labels']['com.docker.compose.project'] == PROJECT
assert ready(), 'Proxy must be ready before disruption'
old_ip = server['NetworkSettings']['Networks'][NETWORK]['IPAddress']
holder = PROJECT + '-ip-reservation-' + uuid.uuid4().hex[:8]
image = dashboard['Image']
created = False
removed = False
try:
    run(*COMPOSE, 'stop', 'server')
    run(*COMPOSE, 'rm', '-f', 'server')
    removed = True
    run('docker', 'run', '-d', '--name', holder, '--network', NETWORK, '--ip', old_ip,
        '--entrypoint', 'sleep', image, '300')
    created = True
    run(*COMPOSE, 'up', '-d', '--no-deps', 'server')
    new_ip = inspect(SERVER)['NetworkSettings']['Networks'][NETWORK]['IPAddress']
    assert new_ip != old_ip, (old_ip, new_ip)
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        try:
            if ready():
                break
        except Exception:
            pass
        time.sleep(2)
    else:
        raise AssertionError('Proxy did not recover within 120 seconds')
    current_dashboard = inspect(DASHBOARD)
    assert current_dashboard['Id'] == dashboard['Id']
    assert current_dashboard['State']['StartedAt'] == dashboard['State']['StartedAt']
    print(json.dumps({'nginx_recreation': 'passed', 'old_ip': old_ip, 'new_ip': new_ip,
                      'dashboard_restarted': False}))
finally:
    if created:
        run('docker', 'rm', '-f', holder)
    if removed:
        run(*COMPOSE, 'up', '-d', '--no-deps', 'server')
