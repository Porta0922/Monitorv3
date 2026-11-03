//! Authoritative snapshots per source. A failed source must never imply removals.
use serde_json::{json, Value};

pub async fn scan() -> Vec<Value> {
    #[cfg(windows)]
    { return tokio::task::spawn_blocking(scan_windows).await.unwrap_or_else(|_| vec![failure("windows-machine-64", "Scanner failed"), failure("windows-machine-32", "Scanner failed")]); }
    #[cfg(target_os="linux")]
    {
        let result = tokio::time::timeout(std::time::Duration::from_secs(30),
            tokio::process::Command::new("/usr/bin/dpkg-query")
                .args(["-W", "-f=${binary:Package}\t${Version}\t${db:Status-Status}\n"])
                .kill_on_drop(true).output()).await;
        let output = match result {
            Ok(Ok(out)) if out.status.success() => out,
            _ => return vec![failure("linux-dpkg", "dpkg-query unavailable, failed or timed out")],
        };
        let text = match String::from_utf8(output.stdout) { Ok(text) => text, Err(_) => return vec![failure("linux-dpkg", "Invalid package encoding")] };
        let mut apps = Vec::new();
        for line in text.lines() {
            let columns: Vec<_> = line.split('\t').collect();
            if columns.len()!=3 { return vec![failure("linux-dpkg", "Incomplete package listing")]; }
            if columns[2]!="installed" { continue; }
            apps.push(json!({"source_key":columns[0],"app_name":columns[0],"version":columns[1]}));
        }
        return vec![snapshot("linux-dpkg", apps)];
    }
    #[cfg(not(any(windows,target_os="linux")))]
    { Vec::new() }
}
fn failure(scope: &str, error: &str) -> Value { json!({"scope":scope,"complete":false,"error":error,"apps":[]}) }
fn snapshot(scope: &str, mut apps: Vec<Value>) -> Value {
    if apps.len()>50000 { return failure(scope,"Source exceeds 50000 entries; snapshot not applied"); }
    apps.sort_by(|a,b|a["source_key"].as_str().cmp(&b["source_key"].as_str()));
    json!({"scope":scope,"complete":true,"apps":apps})
}
#[cfg(windows)]
fn scan_windows() -> Vec<Value> {
    use winreg::{RegKey, enums::{HKEY_LOCAL_MACHINE,KEY_READ,KEY_WOW64_64KEY,KEY_WOW64_32KEY}};
    let root = RegKey::predef(HKEY_LOCAL_MACHINE);
    [("windows-machine-64",KEY_WOW64_64KEY),("windows-machine-32",KEY_WOW64_32KEY)].into_iter().map(|(scope,view)| {
        let key = match root.open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",KEY_READ|view) {
            Ok(key)=>key,
            Err(error) if error.kind()==std::io::ErrorKind::NotFound => return snapshot(scope,Vec::new()),
            Err(_)=>return failure(scope,"Cannot open machine uninstall registry"),
        };
        let mut apps=Vec::new();
        for entry in key.enum_keys() {
            let name=match entry {Ok(name)=>name,Err(_)=>return failure(scope,"Cannot enumerate uninstall registry")};
            let app=match key.open_subkey_with_flags(&name,KEY_READ|view) {Ok(key)=>key,Err(_)=>return failure(scope,"Cannot read an uninstall entry")};
            let display: String = match app.get_value("DisplayName") {
                Ok(name)=>name,
                Err(error) if error.kind()==std::io::ErrorKind::NotFound =>continue,
                Err(_)=>return failure(scope,"Cannot read application name"),
            };
            if display.trim().is_empty() {continue;}
            let version: Option<String> = match app.get_value("DisplayVersion") {
                Ok(version)=>Some(version),
                Err(error) if error.kind()==std::io::ErrorKind::NotFound =>None,
                Err(_)=>return failure(scope,"Cannot read application version"),
            };
            apps.push(json!({"source_key":name.to_lowercase(),"app_name":display,"version":version}));
        }
        snapshot(scope,apps)
    }).collect()
}
