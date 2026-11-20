#!/usr/bin/env python3
"""Reject mismatched native release inputs before packaging. Python 3.10+."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

def source_version(root):
    # Cargo parses TOML and rejects a stale lockfile. Works with Ubuntu 22.04's
    # Python without downloading a TOML parser or changing the system Python.
    data = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'],
        cwd=root, text=True))
    matches = [p['version'] for p in data['packages'] if p['name'] == 'activity-monitor-agent']
    if len(matches) != 1:
        raise RuntimeError('Expected exactly one agent workspace package')
    return matches[0]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--os', required=True, choices=['windows', 'linux'])
    parser.add_argument('--arch', required=True, choices=['x86_64', 'aarch64'])
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    version = source_version(root)
    if args.version != version:
        parser.error('Package version must match agent/Cargo.toml and Cargo.lock')
    binary = args.binary.resolve(strict=True)
    with binary.open('rb') as source:
        digest = hashlib.sha256()
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
        actual = digest.hexdigest()
    if actual != args.sha256.lower():
        parser.error('Binary SHA256 mismatch')
    # Only run a trusted, locally built artifact on its native build platform.
    result = subprocess.run([str(binary), '--version-json'], capture_output=True,
                            text=True, check=True, timeout=15)
    metadata = json.loads(result.stdout)
    expected = dict(name='activity-monitor-agent', version=version, os=args.os, arch=args.arch)
    if metadata != expected:
        parser.error('Binary version/platform differs from package inputs')
    print(json.dumps(expected, sort_keys=True))

if __name__ == '__main__':
    main()
