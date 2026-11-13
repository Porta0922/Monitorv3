#!/usr/bin/env python3
"""Copy only completed encrypted backups using a DevOps-provisioned rclone remote.

Ciphertexts are downloaded and SHA-256 checked before publishing the manifest.
No recovery key is needed. No remote deletion or retention is performed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from backup import FILES, digest, regular, private_directory, trusted_parents, deadline, stop


def remote_digest(command, path, timeout):
    process = subprocess.Popen(command + ['cat', path], stdout=subprocess.PIPE,
                               stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL)
    timer = deadline(process, timeout)
    value = hashlib.sha256()
    try:
        for chunk in iter(lambda: process.stdout.read(1024 * 1024), b''): value.update(chunk)
        if process.wait(timeout=timeout): raise RuntimeError('Remote verification failed')
        return value.hexdigest()
    finally:
        timer.cancel()
        process.stdout.close()
        stop(process)


def publish(directory, remote, command, timeout):
    if not re.fullmatch(r'[A-Za-z0-9_-]+:[A-Za-z0-9_./-]*', remote) or '..' in remote.partition(':')[2].split('/'):
        raise ValueError('Use a named rclone remote and a simple destination path')
    if not re.fullmatch(r'[A-Za-z0-9_-]+', directory.name): raise ValueError('Invalid backup directory name')
    private_directory(directory)
    manifest_path = directory / 'manifest.json'
    regular(manifest_path)
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    if manifest.get('format') != 1 or manifest.get('status') != 'complete' or set(manifest.get('files', {})) != set(FILES):
        raise ValueError('Incomplete backup')
    expected = {}
    for name in FILES:
        path = directory / name
        regular(path)
        metadata = manifest['files'][name]
        expected[name] = metadata['sha256']
        if path.stat().st_size != metadata['bytes'] or digest(path) != expected[name]:
            raise ValueError('Local backup changed or is incomplete')
    target = remote.rstrip('/') + '/' + directory.name
    # Upload only the fixed allowlist; never copy adjacent identity/config files.
    for name in FILES:
        subprocess.run(command + ['copyto', str(directory / name), target + '/' + name, '--immutable'],
                       check=True, timeout=timeout, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                       stdin=subprocess.DEVNULL)
        if remote_digest(command, target + '/' + name, timeout) != expected[name]:
            raise RuntimeError('Remote ciphertext checksum mismatch; manifest not published')
    if manifest_path.read_bytes() != manifest_bytes: raise RuntimeError('Manifest changed during transfer')
    subprocess.run(command + ['copyto', str(manifest_path), target + '/manifest.json', '--immutable'],
                   check=True, timeout=timeout, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                   stdin=subprocess.DEVNULL)
    if remote_digest(command, target + '/manifest.json', timeout) != hashlib.sha256(manifest_bytes).hexdigest():
        raise RuntimeError('Remote manifest verification failed')
    return directory.name


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backup-dir', type=Path, required=True)
    parser.add_argument('--rclone-config', type=Path, required=True)
    parser.add_argument('--remote', required=True)
    parser.add_argument('--timeout-seconds', type=int, default=1800)
    args = parser.parse_args()
    if os.name != 'posix' or os.geteuid() != 0: parser.error('Run as root on the Linux backup host')
    if not args.backup_dir.is_absolute() or not args.rclone_config.is_absolute(): parser.error('Use absolute paths')
    if not 60 <= args.timeout_seconds <= 21600: parser.error('timeout-seconds must be 60..21600')
    trusted_parents(args.backup_dir)
    trusted_parents(args.rclone_config)
    regular(args.rclone_config)
    if args.rclone_config.stat().st_mode & 0o077: parser.error('rclone config must have mode 0600')
    name = publish(args.backup_dir, args.remote,
                   ['rclone', '--config', str(args.rclone_config), '--log-level', 'ERROR'], args.timeout_seconds)
    print(json.dumps({'backup': name, 'remote_ciphertexts_verified': True, 'restore_verified': False}))


if __name__ == '__main__':
    try: main()
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.SubprocessError):
        raise SystemExit('Encrypted backup transfer failed. Keep local copies and investigate privately. No credentials printed.')
