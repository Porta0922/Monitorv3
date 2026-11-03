use chrono::Utc;
use sysinfo::{System, SystemExt, ProcessExt, PidExt, CpuExt, DiskExt, NetworkExt, NetworksExt};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

/// Maximum number of entries in the hash cache before eviction
const MAX_CACHE_SIZE: usize = 5000;

/// Global cache for executable hashes to avoid redundant I/O and CPU usage.
/// Uses a VecDeque to track insertion order for FIFO eviction when the cache exceeds MAX_CACHE_SIZE.
static EXE_HASH_CACHE: OnceLock<Arc<Mutex<(HashMap<String, String>, VecDeque<String>)>>> = OnceLock::new();

fn get_hash_cache() -> Arc<Mutex<(HashMap<String, String>, VecDeque<String>)>> {
    EXE_HASH_CACHE.get_or_init(|| Arc::new(Mutex::new((HashMap::new(), VecDeque::new())))).clone()
}

pub fn calculate_file_hash_with_cache(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cache = get_hash_cache();
    
    // Check cache first
    {
        let guard = cache.lock().unwrap();
        if let Some(hash) = guard.0.get(path) {
            if hash == "ERROR" || hash == "FILE_TOO_LARGE" {
                return Err(format!("Previously failed to hash: {}", hash).into());
            }
            return Ok(hash.clone());
        }
    }

    // Hash the file
    let hash_result = calculate_file_hash(path);
    
    // Update cache with eviction
    let final_hash = match hash_result {
        Ok(h) => h,
        Err(e) => {
            let error_msg = e.to_string();
            let cache_val = if error_msg.contains("too large") {
                "FILE_TOO_LARGE".to_string()
            } else {
                "ERROR".to_string()
            };
            let mut guard = cache.lock().unwrap();
            if guard.0.len() >= MAX_CACHE_SIZE {
                if let Some(oldest) = guard.1.pop_front() {
                    guard.0.remove(&oldest);
                }
            }
            guard.0.insert(path.to_string(), cache_val);
            guard.1.push_back(path.to_string());
            return Err(e);
        }
    };

    {
        let mut guard = cache.lock().unwrap();
        // Evict oldest entry if cache is full
        if guard.0.len() >= MAX_CACHE_SIZE {
            if let Some(oldest) = guard.1.pop_front() {
                guard.0.remove(&oldest);
            }
        }
        guard.0.insert(path.to_string(), final_hash.clone());
        guard.1.push_back(path.to_string());
    }
    
    Ok(final_hash)
}

#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub exe_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WindowCapture {
    pub app_name: String,
    pub window_title: String,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OpenAppSnapshot {
    pub app_name: String,
    pub primary_title: String,
    pub window_count: u32,
    pub exe_path: Option<String>,
    pub exe_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeResourceSnapshot {
    pub system_metrics: serde_json::Value,
    pub cpu_percent: Option<f64>,
    pub memory_used_mb: Option<f64>,
    pub memory_percent: Option<f64>,
    pub top_process_name: Option<String>,
    pub top_process_cpu_percent: Option<f64>,
    pub top_process_memory_mb: Option<f64>,
}

pub struct ResourceMonitor {
    sys: System,
    previous: Option<std::time::Instant>,
    network_totals: HashMap<String, (u64, u64)>,
}

pub struct MonitoringLoop {
    sys: System,
}

impl MonitoringLoop {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    /// Capture all running processes
    pub fn capture_processes(&mut self) -> Vec<ProcessSnapshot> {
        self.sys.refresh_processes();
        
        let mut processes = Vec::new();
        for (pid, process) in self.sys.processes() {
            let exe_path = {
                let p = process.exe();
                p.to_string_lossy().to_string()
            };
            let exe_hash = if !exe_path.is_empty() {
                calculate_file_hash_with_cache(&exe_path).ok()
            } else {
                None
            };

            processes.push(ProcessSnapshot {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                exe_path: Some(exe_path),
                exe_hash,
            });
        }
        
        tracing::info!("📊 Captured {} processes on Linux", processes.len());
        processes
    }

    /// Capture visible user-facing apps that currently have open windows.
    pub fn capture_open_apps(&mut self) -> Vec<OpenAppSnapshot> {
        crate::capture_diagnostics::record("running_apps","no_observation",None);
        #[cfg(target_os="windows")] let apps=capture_open_apps_windows(&mut self.sys);
        #[cfg(target_os="macos")] let apps=capture_open_apps_macos(&mut self.sys);
        #[cfg(target_os="linux")] let apps={
            let mut apps=Vec::new();
            match xcap::Window::all() {
                Ok(windows)=>for window in windows {
                    match (window.app_name(),window.title()) {
                        (Ok(app_name),Ok(title)) if !app_name.is_empty() && !title.is_empty()=>apps.push(OpenAppSnapshot{app_name,primary_title:title,window_count:1,exe_path:None,exe_hash:None}),
                        (Err(error),_)=>crate::capture_diagnostics::record("running_apps","capture_error",Some(&error.to_string())),
                        (_,Err(error))=>crate::capture_diagnostics::record("running_apps","capture_error",Some(&error.to_string())),
                        _=>{}
                    }
                },
                Err(error)=>crate::capture_diagnostics::record("running_apps","capture_error",Some(&error.to_string()))
            } apps
        };
        #[cfg(not(any(target_os="linux",target_os="windows",target_os="macos")))] let apps=Vec::new();
        if !apps.is_empty() && crate::capture_diagnostics::latest("running_apps")["state"]!="capture_error" {crate::capture_diagnostics::record("running_apps","capture_ok",None);}
        apps
    }

    /// Get currently active window using platform APIs
    pub fn capture_active_window(&self) -> Option<WindowCapture> {
        crate::capture_diagnostics::record("window_activity","no_observation",None);
        // Platform-specific implementation
        #[cfg(target_os = "windows")]
        {
            return capture_active_window_windows();
        }
        #[cfg(target_os = "linux")]
        {
            return capture_active_window_linux();
        }
        #[cfg(target_os = "macos")]
        {
            return capture_active_window_macos();
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }

    /// Main monitoring loop - runs every 2 seconds
    pub async fn start(&mut self, interval_secs: u64) {
        let interval = std::time::Duration::from_secs(interval_secs);
        
        loop {
            let _processes = self.capture_processes();
            let _window = self.capture_active_window();
            
            // TODO: Send events to RabbitMQ or offline cache
            
            tokio::time::sleep(interval).await;
        }
    }
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self { sys: System::new(), previous: None, network_totals: HashMap::new() }
    }

    pub fn capture_snapshot(&mut self) -> NodeResourceSnapshot {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.sys.refresh_processes();

        let now = std::time::Instant::now();
        let elapsed = self.previous.map(|last| now.duration_since(last).as_secs_f64());
        let cpu_percent = if elapsed.is_some_and(|seconds| seconds >= 0.2) && !self.sys.cpus().is_empty() {
            Some(f64::from(self.sys.global_cpu_info().cpu_usage()).clamp(0.0, 100.0))
        } else { None };
        // sysinfo 0.29 reports both system and process memory in bytes.
        let total_memory = self.sys.total_memory();
        let used_memory = self.sys.used_memory();
        let memory_used_mb = (total_memory > 0).then_some(used_memory as f64 / 1_048_576.0);
        let memory_percent = (total_memory > 0).then(|| (used_memory as f64 / total_memory as f64 * 100.0).clamp(0.0, 100.0));
        self.sys.refresh_disks_list();
        self.sys.refresh_disks();
        let disks: Vec<_> = self.sys.disks().iter().take(128).map(|disk| serde_json::json!({
            "mount": disk.mount_point().to_string_lossy(),
            "total_bytes": disk.total_space(), "available_bytes": disk.available_space(),
            "used_percent": if disk.total_space() > 0 { Some((disk.total_space().saturating_sub(disk.available_space())) as f64 / disk.total_space() as f64 * 100.0) } else { None },
        })).collect();
        self.sys.refresh_networks_list();
        self.sys.refresh_networks();
        let mut totals = HashMap::new();
        let network: Vec<_> = self.sys.networks().iter().take(128).map(|(name, data)| {
            let received = data.total_received(); let transmitted = data.total_transmitted();
            let rates = self.network_totals.get(name).zip(elapsed).and_then(|(&(rx, tx), seconds)| {
                if seconds > 0.0 && received >= rx && transmitted >= tx {
                    Some(((received-rx) as f64 / seconds, (transmitted-tx) as f64 / seconds))
                } else { None }
            });
            totals.insert(name.clone(), (received, transmitted));
            serde_json::json!({"name":name, "received_bytes":received, "transmitted_bytes":transmitted,
                "receive_bytes_per_second":rates.map(|r|r.0), "transmit_bytes_per_second":rates.map(|r|r.1),
                "rate_state":if rates.is_some(){"available"}else{"warming_up"}})
        }).collect();
        self.network_totals = totals;
        self.previous = Some(now);
        let system_metrics = serde_json::json!({
            "schema_version":1, "sample_interval_seconds":elapsed,
            "hardware":{"logical_cpu_count":self.sys.cpus().len(),
                "cpu_model":self.sys.cpus().first().map(|cpu|cpu.brand()),
                "os_version":self.sys.long_os_version(), "kernel_version":self.sys.kernel_version()},
            "cpu_percent":cpu_percent, "cpu_state":if cpu_percent.is_some(){"available"}else{"warming_up"},
            "memory_used_bytes":(total_memory>0).then_some(used_memory),
            "memory_total_bytes":(total_memory>0).then_some(total_memory), "memory_percent":memory_percent,
            "memory_state":if total_memory>0 {"available"}else{"unavailable"},
            "uptime_seconds":self.sys.uptime(),
            "disks_state":if disks.is_empty(){"unavailable"}else{"available"}, "disks":disks,
            "network_state":if network.is_empty(){"unavailable"}else{"available"}, "network":network,
            "capabilities":crate::runtime::capabilities(),
        });

        let top_process = self
            .sys
            .processes()
            .values()
            .max_by(|a, b| {
                a.cpu_usage()
                    .partial_cmp(&b.cpu_usage())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let num_cpus = self.sys.cpus().len().max(1) as f64;
        let (top_process_name, top_process_cpu_percent, top_process_memory_mb) = if let Some(p) = top_process {
            (
                Some(p.name().to_string()),
                // Normalize by number of logical CPUs so the value stays in 0-100 range
                Some((f64::from(p.cpu_usage()) / num_cpus).clamp(0.0, 100.0)),
                // sysinfo::Process::memory() returns bytes; convert to MB
                Some((p.memory() as f64) / (1024.0 * 1024.0)),
            )
        } else {
            (None, None, None)
        };

        NodeResourceSnapshot {
            system_metrics,
            cpu_percent,
            memory_used_mb,
            memory_percent,
            top_process_name,
            top_process_cpu_percent,
            top_process_memory_mb,
        }
    }
}

#[cfg(target_os = "windows")]
fn capture_open_apps_windows(sys: &mut System) -> Vec<OpenAppSnapshot> {
    use std::path::Path;
    use winapi::shared::minwindef::{BOOL, LPARAM, TRUE};
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{
        EnumWindows, GetShellWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    };

    #[derive(Debug, Clone)]
    struct WindowSeed {
        pid: u32,
        title: String,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if hwnd == GetShellWindow() || IsWindowVisible(hwnd) == 0 {
            return TRUE;
        }

        let title_len = GetWindowTextLengthW(hwnd);
        if title_len <= 0 {
            return TRUE;
        }

        let mut title_buffer = vec![0u16; title_len as usize + 1];
        let copied = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        if copied <= 0 {
            return TRUE;
        }

        let title = String::from_utf16_lossy(&title_buffer[..copied as usize]).trim().to_string();
        if should_skip_window_title(&title) {
            return TRUE;
        }

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return TRUE;
        }

        let windows = &mut *(lparam as *mut Vec<WindowSeed>);
        windows.push(WindowSeed { pid: process_id, title });
        TRUE
    }

    fn should_skip_window_title(title: &str) -> bool {
        let normalized = title.trim().to_lowercase();
        normalized.is_empty()
            || normalized == "program manager"
            || normalized == "windows default lock screen"
            || normalized == "default ime"
            || normalized == "msctfime ui"
            || normalized == "searchhost"
            || normalized == "start"
    }

    fn should_skip_process_name(name: &str) -> bool {
        let normalized = name.trim().to_lowercase();
        let blocked = [
            "explorer.exe",
            "dwm.exe",
            "textinputhost.exe",
            "searchhost.exe",
            "shellexperiencehost.exe",
            "startmenuexperiencehost.exe",
            "lockapp.exe",
            "ctfmon.exe",
            "widgets.exe",
        ];

        blocked.iter().any(|candidate| normalized == *candidate)
    }

    fn display_app_name(process_name: &str, exe_path: &Option<String>) -> String {
        if let Some(path) = exe_path {
            if let Some(file_name) = Path::new(path).file_name().and_then(|value| value.to_str()) {
                return file_name.to_string();
            }
        }
        process_name.to_string()
    }

    let mut windows = Vec::<WindowSeed>::new();
    unsafe {
        if EnumWindows(Some(enum_windows_proc), &mut windows as *mut _ as LPARAM)==0 {
            crate::capture_diagnostics::record("running_apps","capture_error",Some(&std::io::Error::last_os_error().to_string()));
            return Vec::new();
        }
    }

    if windows.is_empty() {
        return Vec::new();
    }

    // 1. Group windows by process ID to avoid redundant sysinfo lookups
    sys.refresh_processes();

    // 2. Map PIDs to their executable info (name, path, hash) to avoid redundant hashing
    let mut pid_to_info: HashMap<u32, (String, Option<String>, Option<String>)> = HashMap::new();
    
    let mut grouped: HashMap<String, OpenAppSnapshot> = HashMap::new();
    for window in windows {
        let info = pid_to_info.entry(window.pid).or_insert_with(|| {
            if let Some(process) = sys.process(sysinfo::Pid::from_u32(window.pid)) {
                let exe_path = {
                    let p = process.exe();
                    let path = p.to_string_lossy().trim().to_string();
                    if path.is_empty() { None } else { Some(path) }
                };
                let process_name = process.name().trim().to_string();
                let app_name = display_app_name(&process_name, &exe_path);
                
                // Calculate hash ONLY ONCE per unique process PID
                let exe_hash = exe_path.as_deref().and_then(|path| calculate_file_hash_with_cache(path).ok());
                (app_name, exe_path, exe_hash)
            } else {
                ("Unknown".to_string(), None, None)
            }
        });

        let (app_name, exe_path, exe_hash) = info;
        if app_name.is_empty() || should_skip_process_name(app_name) {
            continue;
        }

        let group_key = exe_path.clone().unwrap_or_else(|| app_name.to_lowercase());

        let entry = grouped.entry(group_key).or_insert_with(|| {
            OpenAppSnapshot {
                app_name: app_name.clone(),
                primary_title: window.title.clone(),
                window_count: 0,
                exe_path: exe_path.clone(),
                exe_hash: exe_hash.clone(), // Reuse already calculated hash
            }
        });
        entry.window_count += 1;
    }

    let mut apps: Vec<OpenAppSnapshot> = grouped.into_values().collect();
    apps.sort_by(|a, b| {
        b.window_count
            .cmp(&a.window_count)
            .then_with(|| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()))
    });
    apps
}

/// Calculate SHA256 hash of a file.
pub fn calculate_file_hash(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::{Read, BufReader};
    use std::fs::{File, metadata};

    // Check file size to avoid blocking the thread on huge files (e.g., > 100MB)
    if let Ok(meta) = metadata(path) {
        if meta.len() > 100 * 1024 * 1024 {
            return Err("File too large to hash safely".into());
        }
    }

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

// Windows implementation using winapi
#[cfg(target_os = "windows")]
fn capture_active_window_windows() -> Option<WindowCapture> {
    use std::path::Path;
    use winapi::shared::minwindef::DWORD;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winbase::QueryFullProcessImageNameW;
    use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        // Get window title
        let mut title_buffer = [0u16; 256];
        let title_len = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        
        let window_title = if title_len > 0 {
            String::from_utf16_lossy(&title_buffer[..title_len as usize]).to_string()
        } else {
            "Unknown".to_string()
        };

        // Resolve executable from the foreground window process id.
        let mut app_name = "Unknown".to_string();
        let mut process_id: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);

        if process_id != 0 {
            let process_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
            if !process_handle.is_null() {
                let mut exe_buffer = [0u16; 512];
                let mut exe_len: DWORD = exe_buffer.len() as DWORD;

                if QueryFullProcessImageNameW(
                    process_handle,
                    0,
                    exe_buffer.as_mut_ptr(),
                    &mut exe_len,
                ) != 0
                    && exe_len > 0
                {
                    let exe_path = String::from_utf16_lossy(&exe_buffer[..exe_len as usize]);
                    app_name = Path::new(&exe_path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(|value| value.to_string())
                        .unwrap_or(exe_path);
                }

                let _ = CloseHandle(process_handle);
            }
        }

        if app_name=="Unknown" {
            crate::capture_diagnostics::record("window_activity","capture_error",Some("Foreground executable metadata unavailable or access denied"));
        }
        Some(WindowCapture {
            app_name,
            window_title,
            timestamp: Utc::now(),
        })
    }
}

// Linux implementation (uses xprop to query X11/XWayland active window properties)
#[cfg(target_os = "linux")]
fn capture_active_window_linux() -> Option<WindowCapture> {
    fn read(args:&[&str])->Result<String,String> {
        let out=std::process::Command::new("xprop").args(args).output().map_err(|e|e.to_string())?;
        if !out.status.success() {return Err(format!("xprop failed: {}",out.status));}
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
    let result=(||->Result<Option<WindowCapture>,String>{
        let root=read(&["-root","_NET_ACTIVE_WINDOW"])?;
        let id=root.split("window id #").nth(1).and_then(|v|v.split_whitespace().next()).ok_or("Window manager did not report an active-window property")?;
        if id=="0x0" {return Ok(None);}
        let details=read(&["-id",id,"WM_CLASS","_NET_WM_NAME","WM_NAME"])?;
        let mut app=String::new();let mut title=String::new();
        for line in details.lines() {
            let quoted=line.split('"').nth(1).unwrap_or("");
            if line.starts_with("WM_CLASS") {app=quoted.to_string();}
            if line.starts_with("_NET_WM_NAME") || (title.is_empty() && line.starts_with("WM_NAME")) {title=quoted.to_string();}
        }
        if app.is_empty() {return Err("Foreground window class is unavailable".into());}
        Ok(Some(WindowCapture{app_name:app,window_title:title,timestamp:Utc::now()}))
    })();
    match result {Ok(v)=>v,Err(error)=>{crate::capture_diagnostics::record("window_activity","capture_error",Some(&error));None}}
}

// macOS implementation (requires Cocoa framework or osascript)
#[cfg(target_os = "macos")]
fn capture_active_window_macos() -> Option<WindowCapture> {
    use std::process::Command;
    
    // Get active app name
    let app_output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
        .output()
        .ok()?;
        
    let app_name = String::from_utf8_lossy(&app_output.stdout).trim().to_string();
    if app_name.is_empty() {
        return None;
    }
    
    // Get window title
    let script = format!(
        "tell application \"System Events\" to tell process \"{}\" to get name of front window",
        app_name
    );
    let title_output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .ok()?;
        
    let window_title = String::from_utf8_lossy(&title_output.stdout).trim().to_string();
    let window_title = if window_title.starts_with("execution error") || window_title.is_empty() {
        "Unknown".to_string()
    } else {
        window_title
    };
    
    Some(WindowCapture {
        app_name,
        window_title,
        timestamp: chrono::Utc::now(),
    })
}

#[cfg(target_os = "macos")]
fn capture_open_apps_macos(sys: &mut System) -> Vec<OpenAppSnapshot> {
    use std::process::Command;
    
    sys.refresh_processes();

    let output = match Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of every application process whose background only is false")
        .output() {
            Ok(out) => out,
            Err(_) => return Vec::new(),
        };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let apps_names: Vec<&str> = text.split(", ").map(|s| s.trim()).collect();
    
    let mut apps = Vec::new();
    
    for app_name in apps_names {
        if app_name.is_empty() {
            continue;
        }
        
        let mut exe_path = None;
        let mut exe_hash = None;
        
        for (_pid, process) in sys.processes() {
            if process.name().to_lowercase().contains(&app_name.to_lowercase()) {
                let path = process.exe().to_string_lossy().to_string();
                if !path.is_empty() {
                    exe_path = Some(path.clone());
                    exe_hash = calculate_file_hash_with_cache(&path).ok();
                    break;
                }
            }
        }
        
        apps.push(OpenAppSnapshot {
            app_name: app_name.to_string(),
            primary_title: "".to_string(),
            window_count: 1,
            exe_path,
            exe_hash,
        });
    }
    
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_capture() {
        let mut monitor = MonitoringLoop::new();
        let processes = monitor.capture_processes();
        assert!(!processes.is_empty(), "Should capture at least one process");
    }

    #[test]
    fn test_hash_calculation() {
        let result = calculate_file_hash("/tmp/test.txt");
        // Will fail if file doesn't exist, which is ok for this test
        let _ = result;
    }
}
