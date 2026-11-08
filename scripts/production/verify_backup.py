#!/usr/bin/env python3
"""Authenticate encrypted backup streams without writing plaintext to disk.

This verifies encryption, hashes, dump header and archive paths, not DB recovery.
Run a separate restore drill before accepting a backup for production recovery.
"""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
from backup import FILES, digest, regular, private_directory, stop, deadline


def authenticate(path, identity, configuration=False):
    decryptor = subprocess.Popen(['age', '--decrypt', '-i', str(identity), str(path)],
                                 stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    timer = deadline(decryptor, 21600)
    try:
        if configuration:
            count = 0
            with tarfile.open(fileobj=decryptor.stdout, mode='r|') as archive:
                for member in archive:
                    name = PurePosixPath(member.name)
                    if name.is_absolute() or '..' in name.parts or not name.parts or name.parts[0] not in ('config', 'deployment') or not member.isfile():
                        raise ValueError('Unsafe configuration archive member')
                    stream = archive.extractfile(member)
                    while stream.read(1024 * 1024): pass
                    count += 1
            if not count: raise ValueError('Empty configuration archive')
        elif decryptor.stdout.read(5) != b'PGDMP':
            raise ValueError('Not a PostgreSQL custom-format dump')
        # Consume the authenticated stream to EOF, including the final age chunk.
        while decryptor.stdout.read(1024 * 1024): pass
        if decryptor.wait(timeout=60): raise RuntimeError('Backup decryption/authentication failed')
    finally:
        timer.cancel()
        decryptor.stdout.close()
        stop(decryptor)


def verify(directory, identity):
    private_directory(directory)
    regular(identity)
    if identity.stat().st_mode & 0o077: raise ValueError('Recovery identity must have mode 0600 or stricter')
    regular(directory / 'manifest.json')
    manifest = json.loads((directory / 'manifest.json').read_text())
    if manifest.get('format') != 1 or manifest.get('status') != 'complete' or set(manifest.get('files', {})) != set(FILES):
        raise ValueError('Incomplete or unsupported backup manifest')
    for name in FILES:
        path = directory / name
        regular(path)
        expected = manifest['files'][name]
        if path.stat().st_size != expected['bytes'] or digest(path) != expected['sha256']:
            raise ValueError('Backup ciphertext does not match manifest')
        authenticate(path, identity, configuration=name == 'configuration.tar.age')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backup-dir', type=Path, required=True)
    parser.add_argument('--identity-file', type=Path, required=True)
    args = parser.parse_args()
    if os.name != 'posix' or os.geteuid() != 0: parser.error('Run as root on the Linux recovery host')
    if not args.backup_dir.is_absolute() or not args.identity_file.is_absolute(): parser.error('Use absolute paths')
    manifest = verify(args.backup_dir, args.identity_file)
    print(json.dumps({'authenticated': True, 'commit': manifest.get('commit'), 'started_at': manifest.get('started_at'),
                      'database_restore_verified': False}))


if __name__ == '__main__':
    try: main()
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.SubprocessError, tarfile.TarError):
        raise SystemExit('Backup verification failed. Do not restore or discard other backups. No secrets printed.')
