"""Synthetic telemetry test. Run ONLY against the disposable Compose test stack."""
import base64
import datetime
import json
import os
import time
import urllib.request
import urllib.error
import uuid
from http_auth import login_headers

base = os.environ.get('SMOKE_BASE_URL', 'http://127.0.0.1:5173')
broker = os.environ.get('SMOKE_BROKER_URL', 'http://127.0.0.1:15672')


def request(url, payload=None, headers=None):
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(url, data=data, headers={
        'Content-Type': 'application/json', **(headers or {})})
    with urllib.request.urlopen(req, timeout=10) as response:
        return json.load(response)


assert request(base + '/api/health')['status'] == 'ok'
assert request(base + '/api/ready')['status'] == 'ready'
headers = login_headers(base, os.environ.get('SMOKE_ADMIN_USER', 'admin'),
                        os.environ.get('SMOKE_ADMIN_PASSWORD', 'admin'))
# Data endpoints must reject unauthenticated callers on both URL aliases.
for path in ['/api/devices', '/api/overview', '/api/alerts', '/api/audit']:
    try:
        request(base + path)
    except urllib.error.HTTPError as error:
        assert error.code == 401, (path, error.code)
    else:
        raise AssertionError('Unprotected endpoint: ' + path)

device_id = str(uuid.uuid4())
event = {
    'event_id': str(uuid.uuid4()), 'device_id': device_id,
    'hostname': 'refactor-smoke-test', 'event_type': 'input_summary',
    'timestamp': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'payload': {'status': 'active', 'active_seconds': 30,
                'idle_seconds': 0, 'cpu_percent': 42.5,
                'memory_used_mb': 1024, 'memory_percent': 25}}
credentials = os.environ.get('SMOKE_BROKER_CREDENTIALS', 'guest:guest')
broker_headers = {'Authorization': 'Basic ' + base64.b64encode(credentials.encode()).decode()}
message = {'properties': {'delivery_mode': 2}, 'routing_key': 'monitoring.heartbeat',
           'payload': json.dumps(event), 'payload_encoding': 'string'}
for _ in range(2):
    assert request(broker + '/api/exchanges/%2F/monitoring/publish', message,
                   broker_headers)['routed'], 'No queue bound for telemetry'

for attempt in range(30):
    devices = request(base + '/api/devices', headers=headers)['devices']
    if any(d['device_id'] == device_id for d in devices):
        break
    time.sleep(1)
else:
    raise AssertionError('RabbitMQ event did not register a device')

device = request(base + '/api/devices/' + device_id, headers=headers)
assert device['hostname'] == 'refactor-smoke-test'
for attempt in range(30):
    resources = request(base + '/api/resources/' + device_id, headers=headers)
    metrics = resources.get('metrics', [])
    if metrics:
        break
    time.sleep(1)
assert len(metrics) == 1, f'Expected one metric after duplicate delivery: {resources}'
assert metrics[0]['cpu_percent'] == 42.5
assert request(base + '/api/overview', headers=headers)['success']
assert request(base + '/api/metrics/summary', headers=headers)['success']
assert isinstance(request(base + '/api/alerts', headers=headers)['alerts'], list)
# New retention APIs must expose the same sample through the actual HTTP/SQL path.
utc_day = event['timestamp'][:10]
daily_path = f'/api/resources/{device_id}/daily?from={utc_day}&to={utc_day}'
daily = request(base + daily_path, headers=headers)
assert daily['timezone'] == 'UTC'
assert daily['average_basis'] == 'sample'
assert len(daily['days']) == 1, daily
assert daily['days'][0]['resource_samples'] == 1, daily
assert daily['days'][0]['cpu_average'] == 42.5, daily
assert daily['days'][0]['source'] == 'detail', daily
maintenance = request(base + '/api/maintenance/status', headers=headers)
assert maintenance['database_size_bytes'] > 0
assert isinstance(maintenance['history'], list)
for path, auth, expected in [
    (daily_path, {}, 401),
    ('/api/maintenance/status', {}, 401),
    (f'/api/resources/{device_id}/daily?from=2026-01-02&to=2026-01-01', headers, 400),
]:
    try:
        request(base + path, headers=auth)
    except urllib.error.HTTPError as error:
        assert error.code == expected, (path, error.code)
    else:
        raise AssertionError(f'Expected HTTP {expected}: {path}')
print(json.dumps({'status': 'passed', 'device_id': device_id,
                  'checks': ['nginx-api-proxy', 'login', 'rabbitmq-ingestion',
                             'device-parameter-route', 'resource-parameter-route',
                             'duplicate-delivery', 'unauthenticated-data-rejected',
                             'daily-sql-metrics', 'maintenance-status', 'daily-range-rejected']}))
