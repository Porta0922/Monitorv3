#!/usr/bin/env python3
"""Describe immutable release inputs and artifacts; does not build, sign or publish."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()
def digest(path):
    with path.open("rb") as source:
        hasher = hashlib.sha256()
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
        return hasher.hexdigest()
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", action="append", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--environment", required=True, type=Path, help="JSON object: exact compiler, SDK, package tooling and image digests")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if git("status", "--porcelain", "--untracked-files=all"):
        parser.error("Commit release sources first; dirty or untracked inputs are not reproducible")
    environment = json.loads(args.environment.read_text(encoding="utf-8"))
    if not isinstance(environment, dict) or not environment:
        parser.error("Environment must be a nonempty JSON object with exact tool versions")
    artifacts = []
    names = set()
    for path in args.artifact:
        if not path.is_file() or path.name in names:
            parser.error("Artifacts must exist and have unique names")
        names.add(path.name)
        artifacts.append({"name": path.name, "bytes": path.stat().st_size, "sha256": digest(path)})
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", git("show", "-s", "--format=%ct", "HEAD")))
    if epoch < 0:
        parser.error("SOURCE_DATE_EPOCH must be nonnegative")
    inputs = ["agent/Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "dashboard/package-lock.json", "docker-compose.yml"]
    version_match = re.search(r'^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"\s*$', (ROOT / "agent/Cargo.toml").read_text(encoding="utf-8"), re.MULTILINE)
    if not version_match:
        parser.error("Agent Cargo.toml must declare an explicit numeric release version")
    manifest = {"schema_version": 1, "agent_version": version_match.group(1), "commit": git("rev-parse", "HEAD"), "source_date_epoch": epoch,
                "target": args.target, "environment": environment,
                "inputs": {name: digest(ROOT / name) for name in inputs},
                "artifacts": sorted(artifacts, key=lambda item: item["name"])}
    # Exclusive creation: never overwrite a prior release manifest or an artifact.
    with args.output.open("x", encoding="utf-8", newline="\n") as output:
        output.write(json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n")
if __name__ == "__main__":
    main()
