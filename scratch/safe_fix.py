import re

def fix():
    with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
        content = f.read()

    # Find the start of the structs
    structs_start = content.find('#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct SecurityEvent')
    if structs_start == -1:
        structs_start = content.find('pub struct SecurityEvent')
        
    # We can just find the start of `impl Database {`
    impl_start = content.find('impl Database {')
    
    if structs_start != -1 and impl_start != -1:
        # We need to remove from structs_start up to impl_start
        # But wait, `Database` struct is usually at the top. Let's find it.
        db_struct = content.find('pub struct Database {')
        
        imports = """
pub use crate::domains::device::models::*;
pub use crate::domains::activity::models::*;
pub use crate::domains::inventory::models::*;
pub use crate::domains::usb::models::*;
pub use crate::domains::wifi::models::*;
pub use crate::domains::security::models::*;
"""
        # Let's replace the block between the first struct and impl Database {
        # Actually it's easier to just find all `pub struct X { ... }` and remove them except Database.
        
        # A simple regex to remove all structs EXCEPT Database
        struct_pattern = re.compile(r'#\[derive\([^)]+\)\]\npub struct ([a-zA-Z0-9_]+)\s*\{[^}]+\}\n', re.MULTILINE)
        
        def replacer(match):
            name = match.group(1)
            if name == 'Database':
                return match.group(0)
            return ''
            
        new_content = struct_pattern.sub(replacer, content)
        
        # Insert imports right after `use ...`
        use_end = new_content.rfind('use ')
        use_end = new_content.find('\n', use_end) + 1
        
        new_content = new_content[:use_end] + imports + new_content[use_end:]
        
        with open('server/src/postgres_db.rs', 'w', encoding='utf-8') as f:
            f.write(new_content)

def clean_api():
    with open('server/src/api.rs', 'r', encoding='utf-8') as f:
        content = f.read()
        
    start_router = content.find('pub fn create_router')
    if start_router != -1:
        end_router = content.find('}', start_router)
        new_api = content[:start_router]
        
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
"""
    new_api += new_router_code
    
    # We also need to keep the rest of api.rs? No, clean_api is supposed to keep the helpers.
    # If we just replace create_router, all the old routes remain in the file but are unreachable.
    # This avoids compiler errors where other files depend on helpers inside api.rs.
    with open('server/src/api.rs', 'w', encoding='utf-8') as f:
        f.write(new_api)

if __name__ == "__main__":
    fix()
    clean_api()
