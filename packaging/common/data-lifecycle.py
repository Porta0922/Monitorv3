#!/usr/bin/env python3
"""Plan/copy legacy data or purge explicitly selected stopped-agent data, without guessing paths."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import stat
import subprocess
import uuid

FILES = {"device_id.json", "device_identity.json", "session_identity.json", "effective_policy.json", ".env", "session.env",
         "agent_service_cache.db", "agent_user_cache.db", "session_outbox.db", "session-enrollment.json",
         "session-operation.json", "multi-session-operation.json", "package-session-operation.json"}

def no_links(path):
    for item in (path, *path.parents):
        info = item.lstat()
        if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
            raise RuntimeError("Symlink/reparse paths are not supported")
    return path

def stopped():
    if os.name == "nt":
        if not ctypes.windll.shell32.IsUserAnAdmin():
            raise RuntimeError("Run elevated")
        # Fail closed if process enumeration fails. No user-controlled shell fragments.
        script = "$ErrorActionPreference='Stop'; $p=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('activity-monitor-agent.exe','activity-monitor.exe') }); if($p.Count){exit 2}; $s=Get-Service ActivityMonitor -ErrorAction SilentlyContinue; if($s -and $s.Status -ne 'Stopped'){exit 3}"
        subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", script], check=True)
    else:
        if os.geteuid() != 0:
            raise RuntimeError("Run as root")
        state = subprocess.run(["systemctl", "show", "activity-monitor-agent.service", "--property=ActiveState", "--value"], text=True, capture_output=True, check=True).stdout.strip()
        if state not in ("inactive", "failed"):
            raise RuntimeError("Stop the service first")
        for proc in Path("/proc").iterdir():
            if proc.name.isdigit():
                try:
                    name = Path(os.readlink(proc / "exe").removesuffix(" (deleted)")).name
                    if name in ("activity-monitor-agent", "activity-monitor"):
                        raise RuntimeError("Stop all desktop agents first")
                except (FileNotFoundError, ProcessLookupError):
                    pass

def digest(path):
    with path.open("rb") as source:
        hasher = hashlib.sha256()
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
        return hasher.hexdigest()

def inventory(root, included=()):
    no_links(root)
    if not root.is_dir() or root == root.parent:
        raise RuntimeError("Choose a specific data directory")
    entries = []
    for path in sorted(root.iterdir()):
        if path.name not in FILES and path.name not in included:
            continue
        no_links(path)
        if not path.is_file() or path.stat().st_nlink != 1:
            raise RuntimeError("Expected regular files without hardlinks")
        pending = None
        if path.suffix == ".db":
            # readonly URI also reads a committed WAL; no immutable=1 that could hide pending data.
            with sqlite3.connect(path.as_uri() + "?mode=ro", uri=True) as db:
                if db.execute("PRAGMA quick_check").fetchone()[0] != "ok":
                    raise RuntimeError("Database integrity check failed")
                pending = db.execute("SELECT count(*) FROM cache_events WHERE synced=0").fetchone()[0]
        entries.append({"name": path.name, "sha256": digest(path), "pending": pending})
        if path.suffix == ".db":
            for suffix in ("-wal", "-shm", "-journal"):
                companion = path.with_name(path.name + suffix)
                if companion.exists():
                    no_links(companion)
                    if not companion.is_file() or companion.stat().st_nlink != 1:
                        raise RuntimeError("Unexpected SQLite sidecar")
                    entries.append({"name": companion.name, "sha256": digest(companion), "pending": None})
    if not entries:
        raise RuntimeError("No recognized data; no changes made")
    return entries

def private_directory(path):
    path.mkdir(mode=0o700)
    if os.name == "nt":
        subprocess.run(["icacls.exe", str(path), "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F"], check=True, stdout=subprocess.DEVNULL)

def write_private(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as out:
        out.write(data); out.flush(); os.fsync(out.fileno())

def preserve_permissions(source, target):
    if os.name == "nt":
        env=dict(os.environ, MONITOR_MIGRATION_SOURCE=str(source), MONITOR_MIGRATION_TARGET=str(target))
        subprocess.run(["powershell.exe","-NoProfile","-NonInteractive","-Command",
            "$ErrorActionPreference='Stop'; Set-Acl -LiteralPath $env:MONITOR_MIGRATION_TARGET -AclObject (Get-Acl -LiteralPath $env:MONITOR_MIGRATION_SOURCE)"],env=env,check=True)
    else:
        info=source.stat()
        os.chown(target,info.st_uid,info.st_gid)
        os.chmod(target,0o700 if target.is_dir() else (stat.S_IMODE(info.st_mode)&0o640))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("plan", "migrate", "purge"))
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--allow-pending-loss", action="store_true")
    parser.add_argument("--include", action="append", default=[], help="Explicit additional basename, e.g. a principal descriptor; repeat identically for plan and action")
    args = parser.parse_args()
    if any(Path(name).name!=name or name in ("", ".", "..") or "/" in name or "\\" in name for name in args.include):
        parser.error("--include accepts basenames only")
    stopped()
    source = no_links(args.source.absolute()).resolve(strict=True)
    entries = inventory(source,args.include)
    if args.action == "plan":
        # Plans contain hashes, never credentials or ciphertext. Put them in an admin-owned directory.
        write_private(args.plan, (json.dumps({"version":1,"source":str(source),"files":entries}, indent=2) + "\n").encode())
        print("Plan written. No source data changed."); return
    no_links(args.plan.absolute())
    plan = json.loads(args.plan.read_text(encoding="utf-8"))
    if plan != {"version":1,"source":str(source),"files":entries}:
        raise RuntimeError("Source differs from the approved plan; create a new plan")
    if args.action == "migrate":
        if not args.destination:
            parser.error("--destination is required for migration")
        destination = args.destination.absolute()
        no_links(destination.parent)
        if destination.exists() or destination == source or source in destination.parents or destination in source.parents:
            raise RuntimeError("Destination must be a new independent directory")
        private_directory(destination)
        # Keep names and bytes, including SQLite sidecars; never recipher or change UUIDs.
        for entry in entries:
            original = source / entry["name"]
            if digest(original) != entry["sha256"]:
                raise RuntimeError("Source changed during migration; partial destination retained for inspection")
            target = destination / entry["name"]
            write_private(target, original.read_bytes())
            if digest(target) != entry["sha256"]:
                raise RuntimeError("Copy hash mismatch")
        write_private(destination / "migration-manifest.json", (json.dumps(plan, indent=2)+"\n").encode())
        if inventory(source,args.include)!=entries:
            raise RuntimeError("Source changed during migration; do not activate this copy")
        for entry in entries:preserve_permissions(source/entry["name"],destination/entry["name"])
        preserve_permissions(source,destination)
        print("Copy complete with original principals/permissions. Source retained. Configure AGENT_DATA_DIR explicitly before starting; machine, cache key and device identity must remain the same.")
    else:
        if any((entry["pending"] or 0) > 0 for entry in entries) and not args.allow_pending_loss:
            raise RuntimeError("Pending events exist; drain them or explicitly pass --allow-pending-loss")
        # Never recurse or remove the directory; delete only reviewed, recognized files.
        for entry in entries:
            path = source / entry["name"]
            no_links(path)
            if path.parent != source or digest(path) != entry["sha256"]:
                raise RuntimeError("Plan changed during purge; stopped")
            path.unlink()
        print("Selected data purged. Unknown files, directory, binaries and registrations retained.")
if __name__ == "__main__":
    main()
