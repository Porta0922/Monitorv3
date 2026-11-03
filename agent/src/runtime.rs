//! Process role and persistent locations. Service managers own process recovery.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
static INPUT_AVAILABLE: AtomicBool = AtomicBool::new(false);
static INPUT_DIAGNOSTIC: std::sync::LazyLock<std::sync::Mutex<serde_json::Value>> = std::sync::LazyLock::new(||std::sync::Mutex::new(serde_json::json!({"state":"not_started","last_error":null})));
pub fn set_input_diagnostic(available: bool, state: &str, error: Option<String>) {
    INPUT_AVAILABLE.store(available,Ordering::Relaxed);
    if let Ok(mut diagnostic)=INPUT_DIAGNOSTIC.lock() {*diagnostic=serde_json::json!({"state":state,"last_error":error,"observed_at":chrono::Utc::now()});}
}
pub fn input_diagnostic() -> serde_json::Value {
    INPUT_DIAGNOSTIC.lock().map(|v|v.clone()).unwrap_or_else(|_|serde_json::json!({"state":"unknown"}))
}
pub fn mark_input_available() { if !INPUT_AVAILABLE.swap(true,Ordering::Relaxed) {set_input_diagnostic(true,"receiving",None);} }
pub fn input_available() -> bool { INPUT_AVAILABLE.load(Ordering::Relaxed) }

pub fn validate_mode() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::var("AGENT_MODE").as_deref() {
        Ok("service" | "interactive" | "session") | Err(std::env::VarError::NotPresent) => {},
        _ => return Err("AGENT_MODE must be service, interactive or session".into()),
    }
    for name in ["AGENT_DATA_DIR", "AGENT_LOG_DIR", "AGENT_CONFIG_FILE"] {
        if let Some(path) = std::env::var_os(name) {
            if !PathBuf::from(path).is_absolute() { return Err(format!("{name} must be absolute").into()); }
        }
    }
    if let Some(config) = std::env::var_os("AGENT_CONFIG_FILE") {
        let config = PathBuf::from(config);
        // Administrative scripts keep this journal until all credential files agree.
        // Session configs are three levels below the common configuration directory.
        for directory in config.ancestors().skip(1).take(3) {
            if directory.join("session-operation.json").try_exists()? || directory.join("multi-session-operation.json").try_exists()? || directory.join("package-session-operation.json").try_exists()? {
                return Err("Pending session credential operation: run the administrative repair command before starting the agent".into());
            }
        }
    }
    if is_session() {
        let config = std::env::var("AGENT_CONFIG_FILE")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = std::fs::metadata(&config)?;
            if metadata.mode() & 0o077 != 0 || metadata.uid() != unsafe { libc::getuid() } {
                return Err("Session config must belong to the current user with permissions 0600".into());
            }
        }
        if !std::path::Path::new(&config).is_file() { return Err("Session config is missing".into()); }
        if cfg!(windows) && std::env::var_os("AGENT_DATA_DIR").is_none() && std::env::var_os("LOCALAPPDATA").is_none() {
            return Err("Session mode requires LOCALAPPDATA or AGENT_DATA_DIR".into());
        }
    }
    if !cfg!(windows) && !crate::is_running_in_session_0() && std::env::var_os("AGENT_DATA_DIR").is_none()
        && std::env::var_os("XDG_DATA_HOME").is_none() && std::env::var_os("HOME").is_none() {
        return Err("Interactive/session mode requires a persistent home directory".into());
    }
    if crate::network_identity::production() {
        let random_secret=|key:&str|->Result<(),Box<dyn std::error::Error>> {
            let value=std::env::var(key)?;
            if value.len()!=64 || !value.bytes().all(|c|c.is_ascii_hexdigit()) || value.bytes().all(|c|c==value.as_bytes()[0]) {return Err(format!("{key} requires a separately generated 32-byte hex secret").into());}Ok(())
        };
        random_secret("AGENT_OFFLINE_CACHE_KEY")?;
        if relay_enabled() || is_session(){random_secret("AGENT_SESSION_RELAY_KEY")?;}
        if !is_session(){let url=reqwest::Url::parse(&std::env::var("AGENT_SERVER_URL")?)?;if url.scheme()!="https"||url.host_str().is_none(){return Err("Production requires HTTPS AGENT_SERVER_URL".into());}}
    }
    Ok(())
}

pub fn is_session() -> bool { std::env::var("AGENT_MODE").as_deref() == Ok("session") }
pub fn relay_enabled() -> bool { std::env::var("AGENT_SESSION_RELAY_ENABLED").as_deref() == Ok("true") }

pub fn data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("AGENT_DATA_DIR") { return PathBuf::from(path); }
    if cfg!(windows) && is_session() {
        return PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA is required for session mode")).join("ActivityMonitorSession");
    }
    if cfg!(windows) {
        return PathBuf::from(std::env::var_os("ProgramData").unwrap_or_else(|| r"C:\ProgramData".into())).join("ActivityMonitor");
    }
    if crate::is_running_in_session_0() { return PathBuf::from("/var/lib/activity-monitor"); }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join(if is_session() { "activity-monitor-session" } else { "activity-monitor" });
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(if is_session() { ".local/share/activity-monitor-session" } else { ".local/share/activity-monitor" })
}

pub fn desktop_kind() -> &'static str {
    if crate::is_running_in_session_0() { return "none"; }
    if cfg!(windows) { return "windows"; }
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland") { return "wayland"; }
    if std::env::var_os("DISPLAY").is_some() { return "x11"; }
    "none"
}

pub fn desktop_capture_available() -> bool {
    matches!(desktop_kind(), "windows" | "x11")
}

pub fn permits_task(name: &str) -> bool {
    if is_session() {
        return match name {
            "window_activity" | "heatmap" | "running_apps" => desktop_capture_available(),
            "session_input" => input_available(),
            _ => false,
        };
    }
    match name {
        "window_activity" | "heatmap" | "running_apps" => desktop_capture_available(),
        _ => true,
    }
}

pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "mode": if is_session() { "session" } else if crate::is_running_in_session_0() { "service" } else { "interactive" },
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "desktop": desktop_kind(),
        "desktop_capture_eligible": desktop_capture_available(),
        "desktop_capture_note": "Eligibility only; collector initialization may fail",
        "input_available": input_available(),
        "input_diagnostic": input_diagnostic(),
        "persistent_cache_required": true
    })
}

pub async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    { tokio::signal::ctrl_c().await }
}

/// Advisory process lock held for the lifetime of the Unix runtime.
#[cfg(unix)]
pub fn lock_instance() -> std::io::Result<std::fs::File> {
    use std::os::fd::AsRawFd;
    std::fs::create_dir_all(data_dir())?;
    let name=if is_session() && std::env::var("AGENT_MULTI_SESSION").as_deref()==Ok("true") {
        use sha2::Digest;
        let session=std::env::var("XDG_SESSION_ID").or_else(|_|std::env::var("DISPLAY"))
            .map_err(|_|std::io::Error::other("Multi-session mode requires XDG_SESSION_ID or DISPLAY"))?;
        format!("session-{:x}.lock",sha2::Sha256::digest(session.as_bytes()))
    } else {"agent.lock".into()};
    let file = std::fs::OpenOptions::new().create(true).write(true).truncate(false)
        .open(data_dir().join(name))?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(file)
}

#[cfg(all(test, unix))]
mod runtime_signal_tests {
    use super::*;
    #[tokio::test]
    async fn sigterm_is_received_as_administrative_shutdown() {
        let mut signal = Box::pin(shutdown_signal());
        assert!(futures::poll!(&mut signal).is_pending());
        assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0);
        tokio::time::timeout(std::time::Duration::from_secs(2), signal).await.unwrap().unwrap();
    }
}
