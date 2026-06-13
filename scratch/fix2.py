import os, re, glob

# 1. Create security/mod.rs if missing
security_mod = 'server/src/domains/security/mod.rs'
if not os.path.exists(security_mod):
    with open(security_mod, 'w', encoding='utf-8') as f:
        f.write("pub mod models;\npub mod routes;\npub mod repository;\n")
    print(f"Created {security_mod}")

# Ensure all other domains have mod.rs too
for domain in ['device', 'activity', 'inventory', 'usb', 'wifi', 'keystroke']:
    modfile = f'server/src/domains/{domain}/mod.rs'
    if not os.path.exists(modfile):
        with open(modfile, 'w', encoding='utf-8') as f:
            f.write("pub mod models;\npub mod routes;\npub mod repository;\n")
        print(f"Created {modfile}")

# 2. Fix all routes.rs that import from crate::api::parse_iso_date
# -> replace with the version from shared or just remove (parse_iso_date lives in api.rs, we need to expose it there too)
# Actually, easiest fix: add parse_iso_date to shared.rs, 
# and replace "use crate::api::{AppState, parse_iso_date}" with two separate imports

SHARED_FN = """
pub fn parse_iso_date(value: Option<&str>) -> Option<chrono::NaiveDate> {
    value.and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
}
"""

with open('server/src/domains/shared.rs', 'r', encoding='utf-8') as f:
    shared = f.read()

if 'pub fn parse_iso_date' not in shared:
    shared = shared + SHARED_FN
    with open('server/src/domains/shared.rs', 'w', encoding='utf-8') as f:
        f.write(shared)
    print("Added parse_iso_date to shared.rs")

# 3. Fix all domain routes.rs files that say "use crate::api::{AppState, parse_iso_date}"
# Replace with separate imports so api only provides AppState
for rs_path in glob.glob('server/src/domains/*/routes.rs'):
    with open(rs_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    changed = False
    
    # Fix: use crate::api::{AppState, parse_iso_date}
    if 'crate::api::{AppState, parse_iso_date}' in content:
        content = content.replace(
            'use crate::api::{AppState, parse_iso_date};',
            'use crate::api::AppState;\nuse crate::domains::shared::{parse_iso_date, ActivityLogFilters, DateLimitQuery, TzQuery, LiveDevicesQuery, ActiveIdleQuery, format_duration, parse_time_bounds, serialize_device};'
        )
        changed = True

    # Fix: use crate::api::{AppState, parse_iso_date, DateLimitQuery}
    if 'crate::api::{AppState, parse_iso_date, DateLimitQuery}' in content:
        content = content.replace(
            'use crate::api::{AppState, parse_iso_date, DateLimitQuery};',
            'use crate::api::AppState;\nuse crate::domains::shared::{parse_iso_date, DateLimitQuery, TzQuery, ActivityLogFilters, format_duration, parse_time_bounds, serialize_device};'
        )
        changed = True

    # Fix any remaining crate::api::parse_iso_date single import
    if 'use crate::api::parse_iso_date' in content:
        content = content.replace(
            'use crate::api::parse_iso_date;',
            'use crate::domains::shared::parse_iso_date;'
        )
        changed = True

    # Also remove the stray DateLimitQuery import from shared if it's now duplicated
    # Remove "use crate::domains::shared::{DateLimitQuery};" if we already have the bigger import
    if 'use crate::domains::shared::{DateLimitQuery};' in content and 'parse_iso_date, DateLimitQuery' in content:
        content = content.replace('use crate::domains::shared::{DateLimitQuery};\n', '')
        changed = True

    # Also make sure shared::* is not conflicting with explicit shared imports
    if 'use crate::domains::shared::*;' in content and 'use crate::domains::shared::{' in content:
        # Remove the specific import, the wildcard covers it
        content = re.sub(r'use crate::domains::shared::\{[^}]+\};\n', '', content)
        changed = True

    if changed:
        with open(rs_path, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"  Fixed {rs_path}")

# 4. Also make sure api.rs exposes AppState publicly
with open('server/src/api.rs', 'r', encoding='utf-8') as f:
    api = f.read()

if 'pub struct AppState' not in api:
    api = api.replace('struct AppState', 'pub struct AppState')
    with open('server/src/api.rs', 'w', encoding='utf-8') as f:
        f.write(api)
    print("Made AppState public in api.rs")

# 5. Fix bad syntax in postgres_db.rs caused by stray `use crate:...` inside a block
with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
    db = f.read()

# Remove any `crate::` path that is incorrectly inside a use block e.g. `use axum::{r#use}` type garbage
# Check for the "axum::r#use" or similar corruption
if 'r#use' in db:
    db = re.sub(r',\s*\n\s*use crate::[^;]+;\n', '\n', db)
    db = re.sub(r'use axum::r#use[^;]*;', '', db)
    with open('server/src/postgres_db.rs', 'w', encoding='utf-8') as f:
        f.write(db)
    print("Cleaned corruption in postgres_db.rs")

print("\nDone! Run: cargo check")
