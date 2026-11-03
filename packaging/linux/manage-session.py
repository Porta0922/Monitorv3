#!/usr/bin/env python3
"""Repair, activate, revoke or rotate the single enrolled desktop session.
All operations require the system agent and desktop agent to be stopped.
Does not delete or recipher the user's outbox.
"""
import argparse
import fcntl
import grp
import importlib.util
import json
import os
from pathlib import Path
import pwd
import re
import secrets
import stat
import subprocess

spec = importlib.util.spec_from_file_location("enrollment", Path(__file__).with_name("enroll-session.py"))
enrollment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(enrollment)
BASE = enrollment.BASE


def read_file(path, owner):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, encoding="utf-8") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != owner or info.st_mode & 0o022:
            raise RuntimeError(f"Unexpected ownership or writable file: {path}")
        return stream.read()


def setting(text, name, required=True):
    assignments = re.findall(r"^\s*(?:export\s+)?" + re.escape(name) + r"\s*=([^\r\n]*)$", text, re.M)
    if len(assignments) > 1 or (required and not assignments):
        raise RuntimeError(f"Missing or ambiguous setting: {name}")
    return assignments[0].strip().strip("\"'") if assignments else None


def key(text, name):
    value = setting(text, name)
    if not re.fullmatch(r"[0-9a-fA-F]{64}", value):
        raise RuntimeError(f"Invalid key: {name}; refusing to replace it")
    return value


def stopped(user):
    result = subprocess.run(["systemctl", "show", "activity-monitor-agent.service", "--property=ActiveState", "--value"], capture_output=True, text=True, check=True)
    if result.stdout.strip() not in ("inactive", "failed"):
        raise RuntimeError("Stop activity-monitor-agent.service before changing session credentials")
    # Inspect executables instead of truncated process names. Do not terminate arbitrary user processes.
    for process in Path('/proc').iterdir():
        if not process.name.isdigit():
            continue
        try:
            if process.stat().st_uid != user.pw_uid:
                continue
            executable = os.readlink(process / 'exe').removesuffix(' (deleted)')
            if Path(executable).name in ('activity-monitor-agent', 'activity-monitor'):
                raise RuntimeError('Stop the enrolled user agent before changing credentials')
        except (FileNotFoundError, ProcessLookupError):
            continue


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('repair', 'activate', 'revoke', 'rotate', 'reenroll', 'migrate'))
    args = parser.parse_args()
    if os.geteuid() != 0:
        parser.error('Run as root')
    lock = os.open('/run/lock/activity-monitor-lifecycle.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    enrollment.root_directory(BASE)
    marker_path = BASE / 'session-enrollment.json'
    journal_path = BASE / 'session-operation.json'
    pending = journal_path.exists() or journal_path.is_symlink()
    if pending and args.action != 'repair':
        raise RuntimeError('An operation is pending; run repair first')
    record = json.loads(read_file(journal_path if pending else marker_path, 0))
    user = pwd.getpwnam(record['user']) if 'user' in record else pwd.getpwuid(record['uid'])
    if user.pw_uid != record['uid'] or user.pw_uid == 0 or user.pw_name == 'activity-monitor':
        raise RuntimeError('Enrollment principal no longer matches the operating system account')
    stopped(user)
    group = grp.getgrnam('activity-monitor').gr_gid
    enrollment.root_directory(BASE / 'sessions')
    directory = BASE / 'sessions' / str(user.pw_uid)
    enrollment.root_directory(directory)
    config = BASE / '.env'
    session = directory / 'session.env'
    original = read_file(config, 0)
    if pending:
        if record.get('version') != 1 or record.get('final_state') not in ('prepared', 'active', 'revoked'):
            raise RuntimeError('Unknown operation journal; refusing automatic recovery')
        plan = record
        key(plan['service_text'], 'AGENT_SESSION_RELAY_KEY')
        if plan['session_text'] is not None:
            key(plan['session_text'], 'AGENT_SESSION_RELAY_KEY')
            if not setting(plan['session_text'], 'AGENT_OFFLINE_CACHE_KEY'):
                raise RuntimeError('Missing journal cache key')
    else:
        current = read_file(session, user.pw_uid)
        relay_key = key(current, 'AGENT_SESSION_RELAY_KEY')
        # Pin the legacy cache secret BEFORE transport rotation. Preserve explicit custom cache keys verbatim.
        cache_key = setting(current, 'AGENT_OFFLINE_CACHE_KEY', False) or relay_key
        if not cache_key:
            raise RuntimeError('Empty cache key; refusing to replace it')
        port = int(setting(current, 'AGENT_SESSION_RELAY_PORT'))
        if not 1 <= port <= 65535:
            raise RuntimeError('Invalid relay port')
        if args.action in ('activate', 'rotate') and record.get('state') == 'revoked':
            raise RuntimeError('Revoked session must be rotated with an explicit re-enrollment procedure')
        if args.action == 'reenroll' and record.get('state') != 'revoked':
            raise RuntimeError('Re-enrollment requires a revoked session')
        service_key = key(original, 'AGENT_SESSION_RELAY_KEY')
        if args.action != 'migrate' and record.get('state') != 'revoked' and service_key.lower() != relay_key.lower():
            raise RuntimeError('Unjournaled key mismatch; refusing to guess which credential is correct')
        enabled = args.action == 'activate' or (args.action in ('rotate', 'repair') and record.get('state') == 'active')
        new_key = secrets.token_hex(32) if args.action in ('rotate', 'revoke', 'reenroll', 'migrate') else service_key
        cache_settings = {} if setting(current, 'AGENT_OFFLINE_CACHE_KEY', False) is not None else {'AGENT_OFFLINE_CACHE_KEY': cache_key}
        session_text = enrollment.update_env(current, {**cache_settings, 'AGENT_SESSION_RELAY_KEY': new_key}) if args.action != 'revoke' and (record.get('state') != 'revoked' or args.action in ('reenroll', 'migrate')) else None
        final_state = 'revoked' if args.action == 'revoke' or (record.get('state') == 'revoked' and args.action not in ('reenroll', 'migrate')) else ('active' if enabled else 'prepared')
        plan = {'version': 1, 'action': args.action, 'uid': user.pw_uid, 'user': user.pw_name, 'port': port,
                'service_text': enrollment.update_env(original, {'AGENT_SESSION_RELAY_ENABLED': 'true' if enabled else 'false', 'AGENT_SESSION_RELAY_KEY': new_key, 'AGENT_SESSION_RELAY_PORT': port}),
                'session_text': session_text, 'final_state': final_state}
        enrollment.atomic_write(journal_path, json.dumps(plan) + '\n', 0, 0, 0o600)
    # Each replacement is atomic; the journal makes the multi-file operation resumable.
    # Both processes must remain stopped until completion.
    if plan['session_text'] is not None:
        enrollment.atomic_write(session, plan['session_text'], user.pw_uid, user.pw_gid, 0o600)
    enrollment.atomic_write(config, plan['service_text'], 0, group, 0o640)
    enrollment.atomic_write(marker_path, json.dumps({'state': plan['final_state'], 'uid': user.pw_uid,
        'user': user.pw_name, 'port': plan['port'], 'config': str(session)}) + '\n', 0, 0, 0o600)
    journal_path.unlink()
    fd = os.open(BASE, os.O_RDONLY | os.O_DIRECTORY)
    try: os.fsync(fd)
    finally: os.close(fd)
    print(f"Session state: {plan['final_state']}. Services remain stopped. Identity and outbox retained.")


if __name__ == '__main__':
    main()
