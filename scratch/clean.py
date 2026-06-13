import re

def clean_api():
    with open('server/src/api.rs', 'r', encoding='utf-8') as f:
        content = f.read()
        
    idx = content.find('async fn list_devices')
    if idx != -1:
        new_api = content[:idx]
    else:
        new_api = content
        
    start_router = new_api.find('pub fn create_router')
    if start_router != -1:
        new_api = new_api[:start_router]
        
    new_router_code = """
pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::permissive();

    Router::new()
        .nest("/devices", crate::domains::device::routes::router())
        .nest("/logs", crate::domains::activity::routes::router())
        .nest("/inventory", crate::domains::inventory::routes::router())
        .nest("/usb", crate::domains::usb::routes::router())
        .nest("/security", crate::domains::security::routes::router())
        .nest("/heatmaps", crate::domains::keystroke::routes::router())
        .with_state(state)
        .layer(cors)
}

use chrono::{NaiveDate, ParseError};
#[derive(Debug, Deserialize)]
pub struct DateLimitQuery {
    pub date: Option<String>,
    pub limit: Option<i64>,
    pub tz_offset_minutes: Option<i32>,
}
pub fn parse_iso_date(date_str: Option<&str>) -> Option<NaiveDate> {
    date_str.and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
}
"""
    new_api += new_router_code
    
    with open('server/src/api.rs', 'w', encoding='utf-8') as f:
        f.write(new_api)

def clean_db():
    with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
        content = f.read()
        
    # We want to keep `pub struct Database` and `impl Database { pub async fn connect(...) { ... } }`
    # We can just cut right after the end of the `connect` function.
    # The first function after connect is usually `pub async fn insert_activity_logs` or similar.
    # Wait, let's find `pub async fn get_device` or `insert_device` or whatever is right after `connect`
    
    # We can use regex to find `pub async fn insert_activity_logs`
    # Actually, the simplest way is to keep the struct and the connect function.
    start_idx = content.find('pub async fn insert_activity_logs')
    if start_idx == -1:
        start_idx = content.find('pub async fn insert_')
    
    # Just to be safe, I will extract Database and connect exactly.
    # connect is long because it has CREATE TABLEs.
    idx_impl = content.find('impl Database {')
    idx_end = content.find('pub async fn insert_activity_logs')
    if idx_end != -1:
        # the connect function ends right before `pub async fn insert_activity_logs`
        # let's find the closing brace before `pub async fn insert_activity_logs`
        brace_idx = content.rfind('}', 0, idx_end)
        
        # We need to keep everything from 0 to brace_idx + 1, plus a closing brace for the impl
        new_db = content[:brace_idx+1] + "\n}\n"
    else:
        new_db = content
        
    # Remove the structs at the top, since they are moved to domains
    # Actually, if we just remove from `pub struct ActivityLog` to `pub struct Database`
    idx_act = content.find('pub struct ActivityLog')
    idx_db = content.find('pub struct Database')
    if idx_act != -1 and idx_db != -1:
        new_db = new_db[:idx_act] + new_db[idx_db:]
        
    with open('server/src/postgres_db.rs', 'w', encoding='utf-8') as f:
        f.write(new_db)

if __name__ == "__main__":
    clean_api()
    clean_db()
