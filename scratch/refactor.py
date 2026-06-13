import os
import re

def extract_block(text, start_idx):
    idx = text.find('{', start_idx)
    if idx == -1: return None, -1
    count = 1
    idx += 1
    in_string = False
    escape = False
    while idx < len(text) and count > 0:
        char = text[idx]
        if in_string:
            if escape:
                escape = False
            elif char == '\\':
                escape = True
            elif char == '"':
                in_string = False
        else:
            if char == '"':
                in_string = True
            elif char == '{':
                count += 1
            elif char == '}':
                count -= 1
        idx += 1
    return text[start_idx:idx], idx

def find_and_extract_functions(text, func_names):
    extracted = []
    for fn_name in func_names:
        # Match `async fn name` or `fn name`
        pattern = r'(pub\s+)?(async\s+)?fn\s+' + re.escape(fn_name) + r'\s*\('
        for match in re.finditer(pattern, text):
            # find the start of the line or the attribute above it?
            # let's just grab from `pub async fn`
            start_idx = match.start()
            # check if there's an attribute like #[axum::debug_handler] or comments before
            # simple version: just start at `pub async fn`
            block, end_idx = extract_block(text, start_idx)
            if block:
                extracted.append(block)
                # replace with empty spaces to preserve line numbers? Or just remove
                # Actually we don't need to remove if we are rewriting the whole file later
    return extracted

def find_and_extract_structs(text, struct_names):
    extracted = []
    for struct_name in struct_names:
        # Check for #[derive(...)] before it
        pattern = r'(#\[derive\([^)]+\)\]\s*)?(pub\s+)?struct\s+' + re.escape(struct_name) + r'\b'
        for match in re.finditer(pattern, text):
            start_idx = match.start()
            block, end_idx = extract_block(text, start_idx)
            if block:
                extracted.append(block)
    return extracted

def process():
    with open('server/src/api.rs', 'r', encoding='utf-8') as f:
        api_text = f.read()
    with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
        db_text = f.read()

    # Domains definition: (domain_name, [api_funcs], [db_structs], [db_funcs])
    domains = {
        'device': (
            ['list_devices', 'get_device', 'update_device', 'register_device', 'get_live_devices'],
            ['Device', 'DeviceTimeTotals', 'LiveDeviceActivity'],
            ['insert_device', 'update_device_heartbeat', 'get_device', 'get_device_by_hwid', 'get_all_devices', 'update_device_status', 'get_live_devices']
        ),
        'activity': (
            ['ingest_activity_logs', 'query_activity_logs', 'get_device_logs', 'get_active_vs_idle'],
            ['ActivityLog'],
            ['insert_activity_logs', 'get_activity_logs', 'get_device_activity_logs', 'get_device_time_totals', 'get_device_timeline']
        ),
        'inventory': (
            ['list_all_apps', 'list_device_apps', 'list_device_running_apps', 'get_top_apps'],
            ['InventoryItem', 'RunningAppItem', 'TopApp'],
            ['update_inventory', 'get_inventory', 'get_device_inventory', 'update_running_apps', 'get_running_apps', 'get_top_apps', 'get_device_running_apps']
        ),
        'usb': (
            ['list_usb_events', 'list_device_usb_events'],
            ['UsbEvent'],
            ['insert_usb_event', 'get_usb_events']
        ),
        'security': (
            ['list_security_alerts', 'list_device_alerts', 'resolve_alert', 'record_termination_attempt', 'list_security_events', 'get_security_summary', 'list_device_security_events'],
            ['SecurityEvent', 'SecurityAlert', 'SecuritySummaryRow'],
            ['insert_security_event', 'get_security_events', 'insert_security_alert', 'get_security_alerts', 'resolve_security_alert', 'get_security_summary']
        ),
        'keystroke': (
            ['upload_heatmap', 'get_device_heatmaps', 'get_current_heatmap'],
            [], # heatmaps might not have explicit structs in postgres_db? Or they use raw queries.
            ['insert_keystroke_heatmap', 'get_keystroke_heatmaps', 'get_latest_heatmap']
        ),
    }

    for domain, (api_funcs, db_structs, db_funcs) in domains.items():
        print(f"Processing {domain}...")
        
        # Models
        models_content = "use serde::{Deserialize, Serialize};\nuse uuid::Uuid;\nuse chrono::{DateTime, Utc, NaiveDate};\n\n"
        structs_blocks = find_and_extract_structs(db_text, db_structs)
        models_content += "\n\n".join(structs_blocks)
        
        with open(f'server/src/domains/{domain}/models.rs', 'w', encoding='utf-8') as f:
            f.write(models_content)
            
        # Repository
        repo_content = f"use super::models::*;\nuse sqlx::PgPool;\nuse uuid::Uuid;\nuse chrono::*;\n\n"
        repo_funcs = find_and_extract_functions(db_text, db_funcs)
        # remove `pub async fn name(&self,` and replace with `pub async fn name(pool: &PgPool,`
        for fn_block in repo_funcs:
            fn_block = re.sub(r'&\s*self\s*,', 'pool: &PgPool,', fn_block, count=1)
            fn_block = fn_block.replace('&self.pool', 'pool')
            fn_block = fn_block.replace('self.pool', 'pool')
            repo_content += fn_block + "\n\n"
            
        with open(f'server/src/domains/{domain}/repository.rs', 'w', encoding='utf-8') as f:
            f.write(repo_content)
            
        # Routes
        routes_content = "use axum::{extract::{Query, State, Path}, response::IntoResponse, routing::{get, post, patch}, Json, Router};\n"
        routes_content += "use serde_json::json;\nuse std::sync::Arc;\nuse crate::api::{AppState, parse_iso_date};\nuse uuid::Uuid;\nuse super::models::*;\n\n"
        
        routes_content += "pub fn router() -> Router<Arc<AppState>> {\n    Router::new()\n        // ADD ROUTES HERE\n}\n\n"
        
        api_blocks = find_and_extract_functions(api_text, api_funcs)
        for fn_block in api_blocks:
            # Change state.db.func() to super::repository::func(&state.db.pool)
            for dbf in db_funcs:
                fn_block = re.sub(r'state\.db\.' + dbf + r'\((.*?)\)', r'super::repository::' + dbf + r'(&state.db.pool, \1)', fn_block)
                fn_block = re.sub(r'state\.db\.' + dbf + r'\(\)', r'super::repository::' + dbf + r'(&state.db.pool)', fn_block)
            routes_content += fn_block + "\n\n"
            
        with open(f'server/src/domains/{domain}/routes.rs', 'w', encoding='utf-8') as f:
            f.write(routes_content)

if __name__ == "__main__":
    process()
    print("Done")
