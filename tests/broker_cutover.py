"""Disposable RabbitMQ 3.12 -> 4.2 transfer/reversal rehearsal. No pilot data.

Uses Shovel on-confirm acknowledgements and compares message multisets, including
duplicates and dead letters. This is transport evidence, not application dedup.
Only synthetic credentials/messages; HTTP management binds to loopback, AMQP to
a dedicated Docker network. Production TLS is tested separately by identity-stack.
"""
import base64
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import secrets
import subprocess
import time
import urllib.error
import urllib.request

OLD = 'rabbitmq:3.12-management-alpine@sha256:0b44fbcc3a4bf22d00090f1353127577dbe1fcb109c41669733a9d7ecf6c3a78'
NEW = 'rabbitmq:4.2-management-alpine@sha256:77ca2ba3621a95208918e6e331199a6c81129690bf7fc570dc61b6e64726840a'
QUEUES = ('activity_queue', 'activity_queue.dead')


def docker(*args):
    result = subprocess.run(['docker', *args], capture_output=True, text=True, timeout=120)
    if result.returncode:
        raise RuntimeError('Disposable Docker operation failed: ' + args[0])
    return result.stdout.strip()


def wait_for(check, seconds=180):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            if check():
                return
        except (OSError, ValueError, RuntimeError):
            pass
        time.sleep(1)
    raise RuntimeError('Synthetic broker condition timed out')


def main():
    tag = 'monitor-cutover-' + secrets.token_hex(5)
    network = tag + '-network'
    source, target = tag + '-old', tag + '-new'
    user, password = 'fixture', secrets.token_hex(24)
    ports, created = {}, []
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def api(broker, method, path, body=None):
        request = urllib.request.Request(
            f'http://127.0.0.1:{ports[broker]}/api/{path}',
            data=None if body is None else json.dumps(body).encode(), method=method,
            headers={'Authorization': 'Basic ' + base64.b64encode(f'{user}:{password}'.encode()).decode(),
                     'Content-Type': 'application/json'})
        with opener.open(request, timeout=15) as response:
            raw = response.read()
            return json.loads(raw) if raw else None

    def publish(broker, queue, payload):
        result = api(broker, 'POST', 'exchanges/%2F/amq.default/publish', {
            'properties': {'delivery_mode': 2}, 'routing_key': queue,
            'payload': payload, 'payload_encoding': 'string'})
        assert result['routed'], 'Synthetic message was not routed'

    def snapshot(broker, queue):
        rows = api(broker, 'POST', f'queues/%2F/{queue}/get', {
            'count': 1000, 'ackmode': 'ack_requeue_true', 'encoding': 'base64'})
        return Counter(hashlib.sha256(base64.b64decode(r['payload'])).hexdigest() for r in rows)

    def transfer(origin, destination, queue, name):
        api(destination, 'PUT', f'parameters/shovel/%2F/{name}', {'value': {
            'src-protocol': 'amqp091', 'src-uri': f'amqp://{user}:{password}@{origin}/%2F',
            'src-queue': queue, 'src-delete-after': 'queue-length', 'src-prefetch-count': 10,
            'dest-protocol': 'amqp091', 'dest-uri': f'amqp://{user}:{password}@{destination}/%2F',
            'dest-queue': queue, 'ack-mode': 'on-confirm', 'reconnect-delay': 1}})

    def reconciled(origin, destination, queue):
        # Require both payload equality and no source deliveries in flight.
        return (snapshot(destination, queue) == expected[queue]
                and not snapshot(origin, queue)
                and api(origin, 'GET', f'queues/%2F/{queue}').get('messages_unacknowledged') == 0)

    network_created = False
    try:
        docker('network', 'create', network)
        network_created = True
        for broker, image in ((source, OLD), (target, NEW)):
            docker('create', '--name', broker, '--hostname', broker, '--network', network,
                   '-p', '127.0.0.1::15672', '-e', 'RABBITMQ_DEFAULT_USER=' + user,
                   '-e', 'RABBITMQ_DEFAULT_PASS=' + password, image)
            created.append(broker)
            docker('start', broker)
            ports[broker] = int(docker('port', broker, '15672/tcp').rsplit(':', 1)[1])
            wait_for(lambda: api(broker, 'GET', 'overview'))
            docker('exec', broker, 'rabbitmq-plugins', 'enable', 'rabbitmq_shovel', 'rabbitmq_shovel_management')
            for queue in QUEUES:
                api(broker, 'PUT', f'queues/%2F/{queue}', {'durable': True, 'auto_delete': False, 'arguments': {}})
        expected = {}
        for queue in QUEUES:
            payloads = [json.dumps({'event_id': f'{queue}-{i}', 'synthetic': True}, sort_keys=True) for i in range(30)]
            payloads += payloads[:5]  # legitimate redelivery must not disappear in transport
            expected[queue] = Counter(hashlib.sha256(p.encode()).hexdigest() for p in payloads)
            for payload in payloads:
                publish(source, queue, payload)
            assert snapshot(source, queue) == expected[queue]
            transfer(source, target, queue, 'forward-' + queue)
            wait_for(lambda: reconciled(source, target, queue))
        print('PASS: 70 persistent messages, duplicates and dead letters transferred 3.12 -> 4.2', flush=True)

        # A disconnected producer resumes after cutover; add previously buffered IDs.
        for queue in QUEUES:
            late = json.dumps({'event_id': queue + '-offline', 'synthetic': True}, sort_keys=True)
            publish(target, queue, late)
            expected[queue][hashlib.sha256(late.encode()).hexdigest()] += 1
        docker('restart', target)
        ports[target] = int(docker('port', target, '15672/tcp').rsplit(':', 1)[1])
        wait_for(lambda: api(target, 'GET', 'overview'))
        for queue in QUEUES:
            wait_for(lambda: snapshot(target, queue) == expected[queue])
            transfer(target, source, queue, 'reverse-' + queue)
            wait_for(lambda: reconciled(target, source, queue))
        print('PASS: restart persistence, two delayed events and reversal 4.2 -> 3.12; 72 payloads reconciled', flush=True)
    finally:
        for broker in reversed(created):
            docker('rm', '-fv', broker)
        if network_created:
            docker('network', 'rm', network)
    receipts = Path(__file__).resolve().parents[1] / '.local-test/broker-cutover'
    receipts.mkdir(parents=True, exist_ok=True)
    receipt = {'status': 'passed', 'checked_at': datetime.now(timezone.utc).isoformat(),
               'source_image': OLD, 'target_image': NEW, 'forward_messages': 70,
               'reversed_messages': 72, 'duplicates_preserved': True,
               'dead_letters_preserved': True, 'restart_persistence': True,
               'source_unacknowledged': 0, 'application_deduplication_tested': False,
               'real_agent_outbox_tested': False, 'tls_tested': False,
               'production_approved': False}
    (receipts / (tag + '.json')).write_text(json.dumps(receipt, indent=2), encoding='utf-8')


if __name__ == '__main__':
    main()
