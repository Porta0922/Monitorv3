#!/usr/bin/env python3
"""Prepare one user's session credentials. Does not start services or rotate keys."""
import argparse
import fcntl
import grp
import json
import os
from pathlib import Path
import pwd
import re
import secrets
import stat
import subprocess
import tempfile

BASE = Path("/etc/activity-monitor")

def root_directory(path, mode=0o711):
    if path.is_symlink():
        raise RuntimeError(f"Refusing symlink: {path}")
    path.mkdir(mode=mode, exist_ok=True)
    info = path.stat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
        raise RuntimeError(f"Directory must be owned by root without group/other write: {path}")
    os.chmod(path, mode)


def atomic_write(path, content, uid, gid, mode):
    if path.is_symlink():
        raise RuntimeError(f"Refusing symlink: {path}")
    fd, temporary = tempfile.mkstemp(prefix=".enroll-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
            stream.flush()
            os.fchown(stream.fileno(), uid, gid)
            os.fchmod(stream.fileno(), mode)
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def update_env(text, settings):
    pattern = re.compile(r"^\s*(?:export\s+)?(" + "|".join(settings) + r")\s*=")
    lines = [line for line in text.splitlines() if not pattern.match(line)]
    return "\n".join(lines) + "\n" + "".join(f"{key}={value}\n" for key, value in settings.items())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--user", required=True)
    parser.add_argument("--port", type=int, default=47652)
    args = parser.parse_args()
    if os.geteuid() != 0:
        parser.error("Run as root")
    lifecycle_lock = os.open('/run/lock/activity-monitor-lifecycle.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    fcntl.flock(lifecycle_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if not 1 <= args.port <= 65535:
        parser.error("Port must be 1..65535")
    user = pwd.getpwnam(args.user)
    if user.pw_uid == 0 or user.pw_name == "activity-monitor":
        parser.error("Choose the desktop user, not root or the service account")
    service_group = grp.getgrnam("activity-monitor").gr_gid
    state = subprocess.run(["systemctl", "show", "activity-monitor-agent.service", "--property=ActiveState", "--value"],
                           capture_output=True, text=True, check=True).stdout.strip()
    if state not in ("inactive", "failed"):
        parser.error("Stop activity-monitor-agent before enrollment")
    # All writes happen under root-controlled directories, never inside a user's home.
    root_directory(BASE)
    config = BASE / ".env"
    fd = os.open(config, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, encoding="utf-8") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o037:
            raise RuntimeError("Service config must be root-owned, with at most group read permissions")
        original = stream.read()
    marker = BASE / "session-enrollment.json"
    if marker.exists() or marker.is_symlink():
        raise RuntimeError("Enrollment already exists; explicit migration/revocation is required")
    sessions = BASE / "sessions"
    root_directory(sessions)
    user_dir = sessions / str(user.pw_uid)
    root_directory(user_dir)
    session_config = user_dir / "session.env"
    if session_config.exists() or session_config.is_symlink():
        raise RuntimeError("Existing session config will not be overwritten")
    matches = re.findall(r"^\s*(?:export\s+)?AGENT_SESSION_RELAY_KEY\s*=\s*['\"]?([0-9a-fA-F]{64})['\"]?\s*$", original, re.M)
    assignments = re.findall(r"^\s*(?:export\s+)?AGENT_SESSION_RELAY_KEY\s*=", original, re.M)
    if assignments and (len(assignments) != 1 or len(matches) != 1):
        raise RuntimeError("Existing relay key is invalid or ambiguous; refusing rotation")
    key = matches[0] if matches else secrets.token_hex(32)
    cache_key = secrets.token_hex(32)
    settings = {"AGENT_SESSION_RELAY_ENABLED": "false", "AGENT_SESSION_RELAY_PORT": args.port,
                "AGENT_SESSION_RELAY_KEY": key}
    journal = BASE / 'session-operation.json'
    if journal.exists() or journal.is_symlink():
        raise RuntimeError('Pending session operation; use manage-session.py repair')
    atomic_write(journal, json.dumps({
        'version': 1, 'action': 'prepare', 'uid': user.pw_uid, 'user': user.pw_name,
        'service_text': update_env(original, settings),
        'session_text': f'AGENT_MODE=session\nAGENT_SESSION_RELAY_PORT={args.port}\nAGENT_SESSION_RELAY_KEY={key}\nAGENT_OFFLINE_CACHE_KEY={cache_key}\n',
        'final_state': 'prepared', 'port': args.port
    }) + '\n', 0, 0, 0o600)
    # Reserve before publishing any credential. Interrupted enrollment requires manual repair.
    reservation = os.open(marker, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
    with os.fdopen(reservation, "w", encoding="utf-8") as stream:
        json.dump({"state": "preparing", "uid": user.pw_uid, "user": user.pw_name}, stream)
        stream.flush()
        os.fsync(stream.fileno())
    atomic_write(session_config, f"AGENT_MODE=session\nAGENT_SESSION_RELAY_PORT={args.port}\nAGENT_SESSION_RELAY_KEY={key}\nAGENT_OFFLINE_CACHE_KEY={cache_key}\n",
                 user.pw_uid, user.pw_gid, 0o600)
    atomic_write(config, update_env(original, settings), 0, service_group, 0o640)
    atomic_write(marker, json.dumps({"state": "prepared", "uid": user.pw_uid, "user": user.pw_name,
                                    "config": str(session_config), "port": args.port}) + "\n", 0, 0, 0o600)
    journal.unlink()
    directory = os.open(BASE, os.O_RDONLY | os.O_DIRECTORY)
    try: os.fsync(directory)
    finally: os.close(directory)
    print(f"Prepared session config: {session_config}")
    print("Relay remains disabled. No service, login task or firewall rule was started/changed.")


if __name__ == "__main__":
    main()
