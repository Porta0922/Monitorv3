"""Read-only preflight of Git candidate files. Does not stage, commit, push or inspect remote history."""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def git_paths(*args):
    output = subprocess.check_output(["git", *args, "-z"], cwd=ROOT)
    return {p.decode("utf-8") for p in output.split(b"\0") if p}

files = git_paths("ls-files", "--cached") | git_paths("ls-files", "--others", "--exclude-standard")
forbidden_dirs = {".local-test", ".production", ".tools", ".wix", "node_modules", "target"}
forbidden_suffixes = {".exe", ".dll", ".pdb", ".msi", ".msix", ".deb", ".pfx", ".p12", ".key", ".sqlite", ".sqlite3", ".db", ".db-wal", ".db-shm"}
patterns = {
    "legacy_internal_endpoint": re.compile(rb"10[.]30[.]0[.]123"),
    "private_key": re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |DSA |ENCRYPTED )?PRIVATE KEY-----"),
    "github_token": re.compile(rb"(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{50,})"),
    "aws_access_key": re.compile(rb"(?:AKIA|ASIA)[A-Z0-9]{16}"),
}
# Compare exact local secrets without ever printing their values.
secrets = set()
for env_file in [ROOT / ".local-test/.env", ROOT / ".local-test/agent.env"]:
    if env_file.is_file():
        for line in env_file.read_text(encoding="utf-8-sig").splitlines():
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            if any(part in key for part in ("SECRET", "PASSWORD", "TOKEN", "CACHE_KEY", "RELAY_KEY")) and len(value) >= 16:
                secrets.add(value.encode())
issues = []
checked = 0
for name in sorted(files):
    relative = Path(name)
    if forbidden_dirs.intersection(relative.parts) or relative.suffix.lower() in forbidden_suffixes or relative.name == ".env" or (relative.name.startswith(".env.") and relative.name != ".env.example"):
        issues.append({"file": name, "reason": "private_or_generated_artifact"})
        continue
    path = ROOT / relative
    if path.is_symlink():
        issues.append({"file": name, "reason": "symlink_requires_review"})
        continue
    if not path.is_file():
        continue  # Worktree deletion is part of the candidate, not an exported file.
    content = path.read_bytes()
    checked += 1
    if any(value in content for value in secrets):
        issues.append({"file": name, "reason": "exact_local_secret"})
    for reason, pattern in patterns.items():
        if pattern.search(content):
            issues.append({"file": name, "reason": reason})
print(json.dumps({"scope": "tracked plus untracked nonignored working files; not Git history or vulnerability audit", "checked_files": checked, "issues": issues}, indent=2))
sys.exit(bool(issues))
