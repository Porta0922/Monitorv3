"""Linux/root integration fixture: age plus a disposable local PostgreSQL server.

Requires age, age-keygen, pg_dump, pg_restore and psql on PATH. Never target a
deployment: this fixture creates/drops only its UUID-named databases.
"""
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import uuid
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts/production'))
from backup import FILES, digest, encrypt_dump, encrypt_configuration
from verify_backup import verify, authenticate
from publish_backup import publish


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, **kwargs).stdout


def rejected(action):
    try:
        action()
    except (ValueError, RuntimeError, tarfile.TarError, subprocess.SubprocessError):
        return
    raise AssertionError('Invalid backup was accepted')


def main():
    os.umask(0o077)
    for attempt in range(30):
        # The image's temporary initialization server only listens on a socket.
        if subprocess.run(['pg_isready', '-h', '127.0.0.1', '-U', 'postgres'], capture_output=True).returncode == 0: break
        time.sleep(1)
    else: raise RuntimeError('Disposable PostgreSQL did not become ready')
    source = 'backup_source_' + uuid.uuid4().hex
    target = 'backup_restore_' + uuid.uuid4().hex
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        identity, other = root / 'identity', root / 'other'
        for key in (identity, other): run('age-keygen', '-o', str(key))
        recipients = root / 'recipients'
        recipients.write_bytes(run('age-keygen', '-y', str(identity)))
        config = root / 'fixture.json'
        config.write_text('{"fixture": true}')
        try:
            run('createdb', '-U', 'postgres', source)
            run('createdb', '-U', 'postgres', target)
            timescale = run('psql', '-U', 'postgres', '-d', source, '-Atc', "SELECT count(*) FROM pg_available_extensions WHERE name='timescaledb'").strip() == b'1'
            run('psql', '-U', 'postgres', '-d', source, '-v', 'ON_ERROR_STOP=1', '-c',
                'CREATE TABLE evidence(id integer PRIMARY KEY, value text); INSERT INTO evidence SELECT n, md5(n::text) FROM generate_series(1,5000) n;')
            if timescale:
                run('psql', '-U', 'postgres', '-d', source, '-v', 'ON_ERROR_STOP=1', '-c', """
                    CREATE EXTENSION IF NOT EXISTS timescaledb;
                    CREATE TABLE telemetry(at timestamptz NOT NULL, device integer, value integer);
                    SELECT create_hypertable('telemetry', 'at', chunk_time_interval => INTERVAL '1 day');
                    INSERT INTO telemetry SELECT '2020-01-01'::timestamptz + n * INTERVAL '1 minute', n % 3, n FROM generate_series(1,5000) n;
                    ALTER TABLE telemetry SET (timescaledb.compress, timescaledb.compress_segmentby='device');
                    SELECT compress_chunk(show_chunks('telemetry'));
                """)
                run('psql', '-U', 'postgres', '-d', target, '-v', 'ON_ERROR_STOP=1', '-c', 'CREATE EXTENSION IF NOT EXISTS timescaledb; SELECT timescaledb_pre_restore();')
            encrypt_dump(['pg_dump', '-U', 'postgres', '-d', source, '-Fc'], recipients, root / FILES[0], 60)
            encrypt_configuration({'config/fixture.json': config}, recipients, root / FILES[1], 60)
            manifest = {'format': 1, 'status': 'complete', 'files': {
                name: {'sha256': digest(root / name), 'bytes': (root / name).stat().st_size} for name in FILES}}
            (root / 'manifest.json').write_text(json.dumps(manifest))
            verify(root, identity)
            rejected(lambda: verify(root, other))
            # Real rclone local backend exercises the transfer protocol without
            # external credentials. It does not certify a production destination.
            remote_root = root / 'remote-fixture'
            remote_root.mkdir()
            remote_config = root / 'rclone.conf'
            remote_config.write_text('[fixture]\ntype = local\n')
            command = ['rclone', '--config', str(remote_config), '--log-level', 'ERROR']
            remote = 'fixture:' + str(remote_root)
            publish(root, remote, command, 60)
            uploaded = remote_root / root.name
            assert sorted(p.name for p in uploaded.iterdir()) == sorted([*FILES, 'manifest.json'])
            for name in [*FILES, 'manifest.json']: assert digest(uploaded / name) == digest(root / name)
            # A destination returning altered bytes must not receive a completion
            # manifest, even when its copy command reports success.
            with patch('publish_backup.remote_digest', return_value='incorrect'):
                rejected(lambda: publish(root, remote + '/corrupt', command, 60))
            assert not (remote_root / 'corrupt' / root.name / 'manifest.json').exists()
            rejected(lambda: publish(root, 'fixture:../escape', command, 60))
            decrypt = subprocess.Popen(['age', '-d', '-i', str(identity), str(root / FILES[0])], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            try:
                restore = subprocess.run(['pg_restore', '-U', 'postgres', '-d', target, '--exit-on-error'], stdin=decrypt.stdout, capture_output=True, timeout=60)
                decrypt.stdout.close()
                assert decrypt.wait(timeout=60) == 0 and restore.returncode == 0
            finally:
                if decrypt.poll() is None: decrypt.kill(); decrypt.wait()
            if timescale:
                run('psql', '-U', 'postgres', '-d', target, '-v', 'ON_ERROR_STOP=1', '-c', 'SELECT timescaledb_post_restore();')
                for query in ('COPY (SELECT * FROM telemetry ORDER BY at) TO STDOUT',
                              "COPY (SELECT count(*) FROM timescaledb_information.chunks WHERE hypertable_name='telemetry' AND is_compressed) TO STDOUT"):
                    assert run('psql', '-U', 'postgres', '-d', source, '-Atc', query) == run('psql', '-U', 'postgres', '-d', target, '-Atc', query)
            query = 'COPY (SELECT * FROM evidence ORDER BY id) TO STDOUT'
            assert run('psql', '-U', 'postgres', '-d', source, '-Atc', query) == run('psql', '-U', 'postgres', '-d', target, '-Atc', query)
            # Recompute the public manifest to prove age authentication detects
            # tampering independently of the manifest checksum.
            ciphertext = root / FILES[0]
            data = bytearray(ciphertext.read_bytes())
            data[-1] ^= 1
            ciphertext.write_bytes(data)
            manifest['files'][FILES[0]]['sha256'] = digest(ciphertext)
            (root / 'manifest.json').write_text(json.dumps(manifest))
            rejected(lambda: verify(root, identity))
            bad = root / 'unsafe.age'
            archive = io.BytesIO()
            with tarfile.open(fileobj=archive, mode='w') as stream:
                member = tarfile.TarInfo('config/../../escape')
                member.size = 1
                stream.addfile(member, io.BytesIO(b'x'))
            bad.write_bytes(run('age', '-R', str(recipients), input=archive.getvalue()))
            rejected(lambda: authenticate(bad, identity, configuration=True))
            rejected(lambda: encrypt_dump([sys.executable, '-c', 'raise SystemExit(1)'], recipients, root / 'failed.age', 60))
            rejected(lambda: encrypt_dump([sys.executable, '-c', 'import time; time.sleep(30)'], recipients, root / 'timeout.age', 0.1))
            print('PASS: encrypted backup, 5000-row PostgreSQL restore, wrong key, tamper, unsafe archive, failed producer, timeout')
            print('PASS: rclone transfer allowlist, remote digest verification and manifest-last failure handling')
            if timescale: print('PASS: Timescale compressed hypertable restored with 5000 rows and compressed chunks preserved')
        finally:
            for name in (source, target): run('dropdb', '-U', 'postgres', '--if-exists', name)


if __name__ == '__main__': main()
