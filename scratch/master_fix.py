"""
Master fix: restores a clean state for all domain files from scratch,
writing correct models.rs, ensuring mod.rs is complete, and fixing postgres_db.rs.
"""
import subprocess, re, os

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
}

ALL_DOMAINS = ['device', 'activity', 'inventory', 'usb', 'wifi', 'security', 'keystroke', 'shared']

def extract_struct_block(text, struct_name):
    pattern = (
        r'((?:#\[[^\]]+\]\s*)*)'
        r'(pub\s+)?struct\s+' + re.escape(struct_name) + r'\s*\{'
    )
    m = re.search(pattern, text)
    if not m:
        return None
    start = m.start()
    brace_idx = text.index('{', m.end() - 1)
    depth = 1
    idx = brace_idx + 1
    while idx < len(text) and depth > 0:
        c = text[idx]
        if c == '{': depth += 1
        elif c == '}': depth -= 1
        idx += 1
    return text[start:idx].strip()

# --- Step 1: Restore domain files from last-clean commit (8e35ad7) -----------
print("Step 1: Restoring domain dirs from git 8e35ad7 ...")
for domain in ALL_DOMAINS:
    domain_path = f'server/src/domains/{domain}'
    if not os.path.exists(domain_path):
        os.makedirs(domain_path, exist_ok=True)
    for rs_file in ['models.rs', 'routes.rs', 'repository.rs']:
        git_path = f'8e35ad7:server/src/domains/{domain}/{rs_file}'
        r = subprocess.run(['git', 'show', git_path], capture_output=True, cwd='.')
        if r.returncode == 0:
            with open(f'{domain_path}/{rs_file}', 'wb') as f:
                f.write(r.stdout)
            print(f"  Restored {domain}/{rs_file}")
        # else: file didn't exist in that commit, skip

# Restore shared.rs (created by us, not in git)
SHARED_CONTENT = """\
use chrono::{DateTime, Utc, Duration};
use serde::Deserialize;
use serde_json::json;
use crate::config::RuntimeConfig;
use crate::domains::device::models::Device;

#[derive(Debug, Deserialize, Default)]
pub struct ActivityLogFilters {
    pub limit: Option<i64>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub hours: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ActiveIdleQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LiveDevicesQuery {
    pub live_only: Option<bool>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TzQuery {
    pub tz_offset_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
pub struct DateLimitQuery {
    pub limit: Option<i64>,
    pub date: Option<String>,
    pub tz_offset_minutes: Option<i32>,
}

pub fn format_duration(seconds: i64) -> String {
    let safe_seconds = seconds.max(0);
    let hours = safe_seconds / 3600;
    let minutes = (safe_seconds % 3600) / 60;
    let rem_seconds = safe_seconds % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, rem_seconds)
    } else {
        format!("{}s", rem_seconds)
    }
}

pub fn parse_time_bounds(filters: &ActivityLogFilters) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let from = filters
        .from
        .as_deref()
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc));
    let to = filters
        .to
        .as_deref()
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc));
    if from.is_none() {
        if let Some(h) = filters.hours {
            if h > 0 { return (Some(Utc::now() - Duration::hours(h)), to); }
        }
    }
    (from, to)
}

pub fn serialize_device(device: Device, config: &RuntimeConfig) -> serde_json::Value {
    let online = device.last_seen > Utc::now() - Duration::seconds(config.online_threshold_seconds.max(1) as i64);
    json!({
        "id": device.id,
        "device_id": device.device_id,
        "hostname": device.hostname,
        "nickname": device.nickname,
        "mac_address": device.mac_address.unwrap_or_else(|| "Unknown".to_string()),
        "created_at": device.created_at.to_rfc3339(),
        "last_seen": device.last_seen.to_rfc3339(),
        "online": online,
        "stale": !online,
        "status": if online { "online" } else { "offline" }
    })
}
"""
with open('server/src/domains/shared.rs', 'w', encoding='utf-8') as f:
    f.write(SHARED_CONTENT)
print("  Wrote shared.rs")

# --- Step 2: Write correct mod.rs -----------------------------------------
print("Step 2: Writing mod.rs ...")
MOD_RS = """\
pub mod shared;
pub mod device;
pub mod activity;
pub mod inventory;
pub mod usb;
pub mod wifi;
pub mod security;
pub mod keystroke;
"""
with open('server/src/domains/mod.rs', 'w', encoding='utf-8') as f:
    f.write(MOD_RS)
print("  Wrote mod.rs")

# --- Step 3: Write correct models.rs for each domain ----------------------
print("Step 3: Extracting structs into domain models ...")
result = subprocess.run(['git', 'show', 'd128ab5:server/src/postgres_db.rs'], capture_output=True, cwd='.')
original_db = result.stdout.decode('utf-8', errors='replace')

domain_structs = {d: [] for d in set(STRUCT_TO_DOMAIN.values())}
for struct_name, domain in STRUCT_TO_DOMAIN.items():
    block = extract_struct_block(original_db, struct_name)
    if block:
        domain_structs[domain].append(block)
        print(f"  [OK] {struct_name} -> {domain}")
    else:
        print(f"  [MISS] {struct_name}")

MODELS_HEADER = "use serde::{Deserialize, Serialize};\nuse uuid::Uuid;\nuse chrono::{DateTime, Utc};\n\n"
for domain, blocks in domain_structs.items():
    path = f'server/src/domains/{domain}/models.rs'
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(MODELS_HEADER + '\n\n'.join(blocks) + '\n')
    print(f"  Wrote {len(blocks)} structs to {domain}/models.rs")

# Write empty models.rs for keystroke if needed
keystroke_models = 'server/src/domains/keystroke/models.rs'
if not os.path.exists(keystroke_models) or os.path.getsize(keystroke_models) == 0:
    with open(keystroke_models, 'w', encoding='utf-8') as f:
        f.write("// No domain-specific models for keystroke\n")

# --- Step 4: Fix postgres_db.rs re-exports and pub pool -------------------
print("Step 4: Patching postgres_db.rs ...")
with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
    db = f.read()

REEXPORTS = """
pub use crate::domains::device::models::*;
pub use crate::domains::activity::models::*;
pub use crate::domains::inventory::models::*;
pub use crate::domains::usb::models::*;
pub use crate::domains::wifi::models::*;
pub use crate::domains::security::models::*;
"""

# Remove duplicate SecuritySummaryRow import lines added by previous scripts
db = re.sub(r'use crate::domains::security::models::SecuritySummaryRow;\n', '', db)
# Remove duplicate pub use blocks
db = re.sub(r'(pub use crate::domains::\w+::models::\*;\n)+', '', db)

# Make pool public
db = db.replace('    pool: PgPool,', '    pub pool: PgPool,')

# Remove DatabaseClone derive if present (caused conflicts before)
db = db.replace('#[derive(Clone)]\npub struct Database', 'pub struct Database')

# Insert re-exports after the first use block (after the last top-level use line)
lines = db.split('\n')
last_use_line = 0
for i, line in enumerate(lines):
    if line.startswith('use ') or line.startswith('// '):
        last_use_line = i
insert_at = last_use_line + 1
lines.insert(insert_at, REEXPORTS)
db = '\n'.join(lines)

with open('server/src/postgres_db.rs', 'w', encoding='utf-8') as f:
    f.write(db)
print("  Patched postgres_db.rs")

# --- Step 5: Fix api.rs routes import in domain routes files  -------------
print("Step 5: Fixing import of shared types in routes files ...")

SHARED_IMPORT = "use crate::domains::shared::*;\n"
ROUTES_NEEDING_SHARED = [
    'server/src/domains/device/routes.rs',
    'server/src/domains/activity/routes.rs',
]
for path in ROUTES_NEEDING_SHARED:
    if os.path.exists(path):
        with open(path, 'r', encoding='utf-8') as f:
            content = f.read()
        if SHARED_IMPORT not in content:
            # insert after first use line
            idx = content.find('\n', content.find('use ')) + 1
            content = content[:idx] + SHARED_IMPORT + content[idx:]
            with open(path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"  Added shared import to {path}")

# Fix api imports in usb/wifi routes
for path in ['server/src/domains/usb/routes.rs', 'server/src/domains/wifi/routes.rs']:
    if os.path.exists(path):
        with open(path, 'r', encoding='utf-8') as f:
            content = f.read()
        # Replace the import line to include DateLimitQuery
        if 'use crate::api::{AppState' in content and 'DateLimitQuery' not in content:
            content = content.replace(
                'use crate::api::{AppState, parse_iso_date};',
                'use crate::domains::shared::{DateLimitQuery};\nuse crate::api::{AppState, parse_iso_date};'
            )
            with open(path, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"  Added DateLimitQuery to {path}")

print("\nAll done! Run: cargo check")
