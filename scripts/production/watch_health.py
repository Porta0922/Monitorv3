#!/usr/bin/env python3
"""Read-only host watchdog; aggregate signals, cooldown and recovery notifications."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import ssl
import subprocess
import tempfile
import time
import urllib.parse
import urllib.request
from backup import private_directory, regular, trusted_parents
from notify_failure import configuration, deliver, NoRedirect

SERVICES = ('postgres', 'rabbitmq', 'server', 'dashboard', 'db-maintenance')
QUEUES = ('activity_queue', 'inventory_queue', 'heartbeat_queue', 'usb_queue',
          'wifi_queue', 'running_apps_queue', 'security_queue')


def command(*args, input=None):
    result = subprocess.run(args, input=input, text=True, stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, timeout=20, check=True)
    return result.stdout.strip()


def snapshot(args):
    findings = []
    def add(code): findings.append({'code': code, 'status': 'alarm'})
    containers = {}
    for service in SERVICES:
        try:
            ids = command('docker', 'ps', '-a', '--filter', 'label=com.docker.compose.project='+args.project,
                          '--filter', 'label=com.docker.compose.service='+service, '--format', '{{.ID}}').splitlines()
            if len(ids) != 1: raise ValueError('Missing or ambiguous service')
            containers[service] = ids[0]
            state = command('docker', 'inspect', '--format', '{{.State.Status}}|{{if .State.Health}}{{.State.Health.Status}}{{end}}', ids[0])
            if state not in ('running|', 'running|healthy'): add('service.'+service)
        except (OSError, ValueError, subprocess.SubprocessError): add('service.'+service)
    try:
        raw = command('docker', 'exec', containers['rabbitmq'], 'rabbitmqctl', '-q', '-t', '10', 'list_queues',
                      '-p', args.vhost, '--formatter=json', 'name', 'messages_ready', 'messages_unacknowledged', 'consumers')
        queues = {item['name']: item for item in json.loads(raw)}
        for name in QUEUES:
            if name not in queues: add('queue.missing.'+name); continue
            queue = queues[name]
            if queue['messages_ready'] + queue['messages_unacknowledged'] >= args.queue_limit: add('queue.backlog.'+name)
            if queue['consumers'] == 0: add('queue.no_consumer.'+name)
            dead = queues.get(name+'.dead')
            if dead and dead['messages_ready'] + dead['messages_unacknowledged'] > 0: add('queue.dead.'+name)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError): add('broker.unobservable')
    try:
        sql = Path(__file__).with_name('health_snapshot.sql').read_text()
        raw = command('docker', 'exec', '-i', containers['postgres'], 'psql', '-X', '-qAt',
                      '-v', 'ON_ERROR_STOP=1', '-U', 'monitor_bootstrap', '-d', 'activity_monitor', input=sql)
        counts = json.loads(raw)
        limits = {'failed_logins': args.auth_limit, 'denied_requests': args.auth_limit,
                  'limited_accounts': 1, 'csrf_rejections': args.auth_limit, 'security_high': 1,
                  'maintenance_failures': 1, 'maintenance_age': args.maintenance_age,
                  'database_bytes': args.database_limit_bytes}
        for key, limit in limits.items():
            if counts[key] >= limit: add('database.'+key)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError): add('database.unobservable')
    try:
        disk = shutil.disk_usage(args.disk_path)
        if disk.free < args.min_free_bytes or disk.free / disk.total * 100 < args.min_free_percent: add('disk.low')
    except OSError: add('disk.unobservable')
    if args.https_url:
        try:
            context = ssl.create_default_context(cafile=args.ca_file)
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect(), urllib.request.HTTPSHandler(context=context))
            with opener.open(args.https_url, timeout=10) as response:
                if response.status != 200: add('https.not_ready')
        except Exception: add('https.unreachable')
    for certificate in args.certificate:
        try: command('openssl', 'x509', '-in', str(certificate), '-noout', '-checkend', str(args.certificate_days * 86400))
        except (OSError, subprocess.SubprocessError): add('certificate.expiring_or_invalid')
    if args.backup_dir:
        try:
            private_directory(args.backup_dir)
            completed = []
            for child in args.backup_dir.iterdir():
                if child.is_symlink() or not child.is_dir(): continue
                private_directory(child)
                manifest = child/'manifest.json'
                if not manifest.exists(): continue
                regular(manifest)
                data = json.loads(manifest.read_text())
                if data.get('status') == 'complete': completed.append(manifest.stat().st_mtime)
            if not completed or time.time()-max(completed) > args.backup_age: add('backup.stale')
        except (OSError, ValueError, TypeError): add('backup.unobservable')
    return sorted({item['code']: item for item in findings}.values(), key=lambda x: x['code'])


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--project', required=True)
    p.add_argument('--vhost', default='/')
    p.add_argument('--state-dir', type=Path, default=Path('/var/lib/monitor-watchdog'))
    p.add_argument('--webhook-config', type=Path, default=Path('/etc/monitor-alert/webhook.json'))
    p.add_argument('--disk-path', type=Path, default=Path('/var/lib/docker'))
    p.add_argument('--min-free-bytes', type=int, default=5*1024**3)
    p.add_argument('--min-free-percent', type=int, default=10)
    p.add_argument('--database-limit-bytes', type=int, default=100*1024**3)
    p.add_argument('--queue-limit', type=int, default=10000)
    p.add_argument('--auth-limit', type=int, default=20)
    p.add_argument('--maintenance-age', type=int, default=7200)
    p.add_argument('--backup-dir', type=Path)
    p.add_argument('--backup-age', type=int, default=7200)
    p.add_argument('--https-url')
    p.add_argument('--ca-file')
    p.add_argument('--certificate', type=Path, action='append', default=[])
    p.add_argument('--certificate-days', type=int, default=14)
    p.add_argument('--reminder-seconds', type=int, default=1800)
    p.add_argument('--report-only', action='store_true', help='No delivery; still reports alarms with exit code 1')
    a = p.parse_args()
    if os.geteuid() != 0: p.error('Run as root on the deployment host')
    if not re.fullmatch(r'[a-z0-9][a-z0-9_-]{0,62}', a.project): p.error('Invalid Compose project')
    if not a.state_dir.is_absolute(): p.error('State directory must be absolute')
    if any(v < 1 for v in (a.min_free_bytes,a.database_limit_bytes,a.queue_limit,a.auth_limit,a.maintenance_age,a.backup_age,a.certificate_days)) or not 1 <= a.min_free_percent <= 99 or a.reminder_seconds < 300: p.error('Invalid thresholds')
    if a.https_url:
        u = urllib.parse.urlsplit(a.https_url)
        if u.scheme != 'https' or not u.hostname or u.username or u.password or u.query or u.fragment: p.error('Use an HTTPS readiness URL without secrets')
    trusted_parents(a.state_dir)
    a.state_dir.mkdir(mode=0o700, exist_ok=True)
    private_directory(a.state_dir)
    lock = os.open(a.state_dir/'lock', os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW, 0o600)
    try:
        fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
        findings = snapshot(a)
        print(json.dumps({'project': a.project, 'findings': findings, 'healthy': not findings}))
        if a.report_only: return int(bool(findings))
        state_path = a.state_dir/'state.json'
        previous = {}
        if state_path.exists():
            regular(state_path)
            previous = json.loads(state_path.read_text())
            if previous.get('project') != a.project: previous = {}
        fingerprint = hashlib.sha256(json.dumps(findings, sort_keys=True).encode()).hexdigest()
        now = time.time()
        changed = fingerprint != previous.get('fingerprint')
        due = bool(findings) and now - previous.get('notified_at', 0) >= a.reminder_seconds
        # Initial healthy observation needs no notification; recovery does.
        if (changed and (findings or previous.get('fingerprint'))) or due:
            config, context = configuration(a.webhook_config)
            if not deliver(config, context, 'monitor-watchdog.service', findings): return 2
        with tempfile.NamedTemporaryFile(mode='w', dir=a.state_dir, delete=False) as output:
            json.dump({'project': a.project, 'fingerprint': fingerprint, 'notified_at': now if changed or due else previous.get('notified_at',0), 'checked_at': now}, output)
            output.flush(); os.fsync(output.fileno())
        os.replace(output.name, state_path)
        return int(bool(findings))
    finally: os.close(lock)


if __name__ == '__main__':
    try: raise SystemExit(main())
    except Exception: raise SystemExit('Watchdog failed; investigate service/configuration. No secrets printed.')
