#!/usr/bin/env python3
"""Create an age-encrypted database/configuration backup on the Linux host.

Requires age, Docker Compose and root. The recovery private key is not required.
No plaintext database dump is written to disk. A manifest is published only after
both encrypted streams finish and configuration remains unchanged.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tarfile
import threading
import uuid

ROOT = Path(__file__).resolve().parents[2]
FILES = ('database.dump.age', 'configuration.tar.age')


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''): value.update(chunk)
    return value.hexdigest()


def regular(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
        raise ValueError('Inputs must be root-owned regular files, not writable by group/others')


def trusted_parents(path):
    for parent in path.absolute().parents:
        info = parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise ValueError('Path parents must be root-owned directories without group/other write access')


def private_directory(path):
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o077:
        raise ValueError('Private directories must be root-owned with mode 0700')


def configuration_files(private, inputs):
    private_directory(private)
    if (private / 'tls-renewal-pending').exists(): raise ValueError('Certificate renewal is pending')
    files = {}
    for path in sorted(private.rglob('*')):
        if path.is_symlink(): raise ValueError('Configuration symlinks are not supported')
        if path.is_dir(): continue
        regular(path)
        files['config/' + path.relative_to(private).as_posix()] = path
    for name, path in inputs.items():
        regular(path)
        files['deployment/' + name] = path
    required = ['accounts/users.json', 'deployment.json', 'endpoint.env', 'init-roles.sql',
                'rabbit-definitions.json', 'rabbitmq.conf', 'secrets/db_password',
                'secrets/database_url', 'secrets/migration_url', 'secrets/rabbitmq_url',
                'secrets/broker_admin_password', 'secrets/maintenance_password', 'secrets/jwt_secret',
                'tls/ca.crt', 'tls/web.crt', 'tls/web.key', 'tls/broker.crt', 'tls/broker.key']
    if any('config/' + name not in files for name in required): raise ValueError('Incomplete production configuration')
    return files


def stop(process):
    if process is not None and process.poll() is None:
        process.kill()
        process.wait()


def deadline(process, seconds):
    def expire():
        try:
            if process.poll() is None: process.kill()
        except ProcessLookupError:
            pass
    timer = threading.Timer(seconds, expire)
    timer.daemon = True
    timer.start()
    return timer


def encrypt_dump(command, recipients, output, timeout):
    producer = encryptor = None
    try:
        with output.open('xb') as encrypted:
            producer = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            encryptor = subprocess.Popen(['age', '--encrypt', '-R', str(recipients)],
                                         stdin=producer.stdout, stdout=encrypted, stderr=subprocess.DEVNULL)
            producer.stdout.close()
            encrypted_result = encryptor.wait(timeout=timeout)
            dump_result = producer.wait(timeout=timeout)
            if encrypted_result or dump_result: raise RuntimeError('Database backup pipeline failed')
            encrypted.flush()
            os.fsync(encrypted.fileno())
    finally:
        stop(encryptor)
        stop(producer)


def encrypt_configuration(files, recipients, output, timeout):
    encryptor = None
    timer = None
    try:
        with output.open('xb') as encrypted:
            encryptor = subprocess.Popen(['age', '--encrypt', '-R', str(recipients)],
                                         stdin=subprocess.PIPE, stdout=encrypted, stderr=subprocess.DEVNULL)
            # Bound blocked pipe writes as well as the final process wait.
            timer = deadline(encryptor, timeout)
            with tarfile.open(fileobj=encryptor.stdin, mode='w|') as archive:
                for name, path in files.items():
                    regular(path)
                    # Follow no symlinks, and archive only explicitly inventoried files.
                    archive.add(path, arcname=name, recursive=False)
            encryptor.stdin.close()
            if encryptor.wait(timeout=timeout): raise RuntimeError('Configuration encryption failed')
            encrypted.flush()
            os.fsync(encrypted.fileno())
    finally:
        if timer is not None: timer.cancel()
        stop(encryptor)


def run(command):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, timeout=60)
    if result.returncode: raise RuntimeError('Deployment inspection failed; no credentials printed')
    return result.stdout.decode().strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--recipients-file', type=Path, required=True)
    parser.add_argument('--project-name', default='monitor-production-candidate')
    parser.add_argument('--env-file', type=Path)
    parser.add_argument('--compose-file', type=Path, action='append', default=[])
    parser.add_argument('--timeout-seconds', type=int, default=1800)
    args = parser.parse_args()
    if os.name != 'posix' or os.geteuid() != 0: parser.error('Run as root on the Linux deployment host')
    if not 60 <= args.timeout_seconds <= 21600: parser.error('timeout-seconds must be 60..21600')
    if not args.output_dir.is_absolute() or not args.recipients_file.is_absolute(): parser.error('Use absolute paths')
    private_directory(args.output_dir)
    regular(args.recipients_file)
    trusted_parents(args.output_dir)
    trusted_parents(args.recipients_file)
    private = ROOT / '.production'
    trusted_parents(private)
    if args.output_dir.resolve().is_relative_to(private.resolve()): parser.error('Backup output must be outside .production')
    command = ['docker', 'compose', '--project-name', args.project_name]
    inputs = {'compose-base.yml': ROOT / 'docker-compose.yml', 'compose-production.yml': ROOT / 'docker-compose.production.yml'}
    if args.env_file:
        inputs['variables.env'] = args.env_file.absolute()
        command += ['--env-file', str(args.env_file.absolute())]
    command += ['-f', str(inputs['compose-base.yml']), '-f', str(inputs['compose-production.yml'])]
    for index, overlay in enumerate(args.compose_file):
        inputs[f'overlay-{index}.yml'] = overlay.absolute()
        command += ['-f', str(overlay.absolute())]
    files = configuration_files(private, inputs)
    for path in inputs.values(): trusted_parents(path)
    before = {name: digest(path) for name, path in files.items()}
    run(command + ['config', '--quiet'])
    container = run(command + ['ps', '-q', 'postgres'])
    if not container or len(container.splitlines()) != 1: raise ValueError('Exactly one running PostgreSQL container is required')
    image = run(['docker', 'inspect', '--format', '{{.Image}}', container])
    commit = run(['git', 'rev-parse', 'HEAD'])
    os.umask(0o077)
    started = datetime.datetime.now(datetime.timezone.utc)
    output = args.output_dir / (started.strftime('%Y%m%dT%H%M%SZ-') + uuid.uuid4().hex[:12])
    output.mkdir(mode=0o700)
    encrypt_dump(command + ['exec', '-T', 'postgres', 'pg_dump', '--username=monitor_bootstrap',
                            '--dbname=activity_monitor', '--format=custom'], args.recipients_file,
                 output / FILES[0], args.timeout_seconds)
    encrypt_configuration(files, args.recipients_file, output / FILES[1], args.timeout_seconds)
    after = {name: digest(path) for name, path in configuration_files(private, inputs).items()}
    if before != after: raise RuntimeError('Configuration changed during backup; no complete manifest published')
    manifest = {'format': 1, 'status': 'complete', 'started_at': started.isoformat(),
                'finished_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'commit': commit, 'project': args.project_name, 'postgres_image_id': image,
                'files': {name: {'sha256': digest(output / name), 'bytes': (output / name).stat().st_size} for name in FILES}}
    pending = output / 'manifest.json.partial'
    with pending.open('x') as stream:
        json.dump(manifest, stream, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(pending, output / 'manifest.json')
    directory = os.open(output, os.O_DIRECTORY)
    try: os.fsync(directory)
    finally: os.close(directory)
    print('Encrypted backup complete: ' + str(output))


if __name__ == '__main__':
    try: main()
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError, tarfile.TarError):
        raise SystemExit('Backup failed; incomplete directories have no complete manifest. Check private configuration and tool availability. No secrets printed.')
