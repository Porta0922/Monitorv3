"""
Full extraction: reads the ORIGINAL postgres_db.rs from git (d128ab5),
extracts every struct with its derive block, and writes them to the
correct domain models.rs file.
"""
import subprocess
import re
import os

# Struct -> domain mapping
STRUCT_TO_DOMAIN = {
    'ActivityLog':          'activity',
    'InventoryItem':        'inventory',
    'RunningAppItem':       'inventory',
    'TopApp':               'inventory',
    'UsbEvent':             'usb',
    'WifiEvent':            'wifi',
    'DeviceTimeTotals':     'device',
    'LiveDeviceActivity':   'device',
    'Device':               'device',
    'Overview':             'device',
    'AuditEvent':           'device',
    'OperationalMetrics':   'device',
    'NodeResourceMetric':   'device',
    'DeviceResourcePeak':   'device',
    'SecurityEvent':        'security',
    'SecurityAlert':        'security',
    'SecuritySummaryRow':   'security',
    # StreamEvent stays in postgres_db (keep-alive helper)
}

def extract_struct_block(text, struct_name):
    """Extract #[derive(...)]\npub struct Name { ... } from text."""
    # Match optional #[derive] + optional #[allow] then struct
    pattern = (
        r'((?:#\[[^\]]+\]\s*)*)'   # any number of attributes
        r'(pub\s+)?struct\s+' + re.escape(struct_name) + r'\s*\{'
    )
    m = re.search(pattern, text)
    if not m:
        return None
    start = m.start()
    # find the matching closing brace
    brace_idx = text.index('{', m.end() - 1)
    depth = 1
    idx = brace_idx + 1
    while idx < len(text) and depth > 0:
        c = text[idx]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
        idx += 1
    return text[start:idx].strip()

# Get original file from git - write to temp file to avoid encoding issues
import tempfile, pathlib
result = subprocess.run(
    ['git', 'show', 'd128ab5:server/src/postgres_db.rs'],
    capture_output=True, cwd='.'
)
original_db = result.stdout.decode('utf-8', errors='replace')

# Build per-domain struct content
domain_structs = {d: [] for d in set(STRUCT_TO_DOMAIN.values())}

for struct_name, domain in STRUCT_TO_DOMAIN.items():
    block = extract_struct_block(original_db, struct_name)
    if block:
        domain_structs[domain].append(block)
        print(f"  [OK] {struct_name} -> {domain}")
    else:
        print(f"  [MISS] {struct_name} not found in original")

HEADER = """\
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};

"""

for domain, blocks in domain_structs.items():
    path = f'server/src/domains/{domain}/models.rs'
    content = HEADER + '\n\n'.join(blocks)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Wrote {len(blocks)} structs to {path}")

print("Done.")
