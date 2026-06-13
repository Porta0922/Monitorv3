import os

def insert_import(file_path, import_stmt):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Insert right after the first `use ` statement
    idx = content.find('use ')
    if idx != -1:
        end_idx = content.find('\n', idx)
        new_content = content[:end_idx+1] + import_stmt + '\n' + content[end_idx+1:]
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(new_content)

def fix_imports():
    insert_import('server/src/domains/device/routes.rs', 'use crate::domains::shared::*;\nuse chrono::*;\nuse std::collections::HashMap;')
    insert_import('server/src/domains/activity/routes.rs', 'use crate::domains::shared::*;\nuse chrono::*;')
    insert_import('server/src/domains/usb/routes.rs', 'use crate::api::DateLimitQuery;')
    insert_import('server/src/domains/wifi/routes.rs', 'use crate::api::DateLimitQuery;')
    insert_import('server/src/postgres_db.rs', 'use crate::domains::security::models::SecuritySummaryRow;')

if __name__ == '__main__':
    fix_imports()
