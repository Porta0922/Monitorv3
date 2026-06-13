import re

def fix_imports():
    # 1. Fix postgres_db.rs
    with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
        content = f.read()
    
    postgres_imports = """
use crate::domains::device::models::*;
use crate::domains::activity::models::*;
use crate::domains::inventory::models::*;
use crate::domains::usb::models::*;
use crate::domains::wifi::models::*;
use crate::domains::security::models::*;
"""
    if "use crate::domains::device::models::*;" not in content:
        idx = content.find('use ')
        idx = content.find('\n', idx) + 1
        content = content[:idx] + postgres_imports + content[idx:]
        with open('server/src/postgres_db.rs', 'w', encoding='utf-8') as f:
            f.write(content)

    # 2. Fix usb/routes.rs and wifi/routes.rs
    for path in ['server/src/domains/usb/routes.rs', 'server/src/domains/wifi/routes.rs']:
        with open(path, 'r', encoding='utf-8') as f:
            content = f.read()
        if "DateLimitQuery" not in content and "use crate::api::" in content:
            content = content.replace("use crate::api::{AppState, parse_iso_date};", "use crate::api::{AppState, parse_iso_date, DateLimitQuery};")
            with open(path, 'w', encoding='utf-8') as f:
                f.write(content)

    # 3. Fix device/routes.rs and activity/routes.rs
    shared_imports = """
use crate::domains::shared::*;
use chrono::{Utc, Duration};
use std::collections::HashMap;
"""
    for path in ['server/src/domains/device/routes.rs', 'server/src/domains/activity/routes.rs']:
        with open(path, 'r', encoding='utf-8') as f:
            content = f.read()
        if "use crate::domains::shared::*" not in content:
            idx = content.find('use ')
            idx = content.find('\n', idx) + 1
            content = content[:idx] + shared_imports + content[idx:]
            with open(path, 'w', encoding='utf-8') as f:
                f.write(content)

if __name__ == '__main__':
    fix_imports()
