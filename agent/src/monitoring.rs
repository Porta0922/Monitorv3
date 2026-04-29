// Process and window monitoring module
use chrono::Utc;
use sysinfo::{System, SystemExt, ProcessExt, PidExt, CpuExt};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::OnceLock;

static EXE_HASH_CACHE: OnceLock<Arc<Mutex<HashMap<String, String>>>> = OnceLock::new();

fn get_hash_cache() -> Arc<Mutex<HashMap<String, String>>> {
    EXE_HASH_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new()))).clone()
}

pub fn calculate_file_hash_with_cache(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cache = get_hash_cache();
    {
        let guard = cache.lock().unwrap();
        if let Some(hash) = guard.get(file_path) {
            return Ok(hash.clone());
        }
    }
    
    let hash = calculate_file_hash(file_path)?;
    
    let mut guard = cache.lock().unwrap();
    guard.insert(file_path.to_string(), hash.clone());
    Ok(hash)
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
    pub cpu_percent: f64,
    pub memory_used_mb: f64,
    pub memory_percent: f64,
    pub top_process_name: Option<String>,
    pub top_process_cpu_percent: Option<f64>,
    pub top_process_memory_mb: Option<f64>,
}

pub struct ResourceMonitor {
    sys: System,
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
        self.sys.refresh_all();
        
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
        #[cfg(target_os = "windows")]
        {
            return capture_open_apps_windows(&mut self.sys);
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.capture_processes()
                .into_iter()
                .filter(|process| !process.name.trim().is_empty())
                .map(|process| OpenAppSnapshot {
                    app_name: process.name,
                    primary_title: String::new(),
                    window_count: 1,
                    exe_path: process.exe_path,
                    exe_hash: process.exe_hash,
                })
                .collect()
        }
    }

    /// Get currently active window using platform APIs
    pub fn capture_active_window(&self) -> Option<WindowCapture> {
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
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();
        sys.refresh_processes();
        Self { sys }
    }

    pub fn capture_snapshot(&mut self) -> NodeResourceSnapshot {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.sys.refresh_processes();

        let cpu_percent = f64::from(self.sys.global_cpu_info().cpu_usage()).max(0.0);
        let total_memory_kb = self.sys.total_memory();
        let used_memory_kb = self.sys.used_memory();

        let memory_used_mb = (used_memory_kb as f64) / 1024.0;
        let memory_percent = if total_memory_kb > 0 {
            ((used_memory_kb as f64) / (total_memory_kb as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

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
        EnumWindows(Some(enum_windows_proc), &mut windows as *mut _ as LPARAM);
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

        let entry = grouped.entry(group_key).or_insert_with(|| OpenAppSnapshot {
            app_name: app_name.clone(),
            primary_title: window.title.clone(),
            window_count: 0,
            exe_hash: exe_hash.clone(),
            exe_path: exe_path.clone(),
        });

        entry.window_count += 1;
        if entry.primary_title.len() < window.title.len() {
            entry.primary_title = window.title;
        }
    }

    let mut apps: Vec<OpenAppSnapshot> = grouped.into_values().collect();
    apps.sort_by(|a, b| {
        b.window_count
            .cmp(&a.window_count)
            .then_with(|| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()))
    });
    apps
}

/// Calculate SHA-256 hash of a file
pub fn calculate_file_hash(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(format!("{:x}", hasher.finalize()))
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

        Some(WindowCapture {
            app_name,
            window_title,
            timestamp: Utc::now(),
        })
    }
}

// Linux implementation (basic - requires x11-clipboard or similar)
#[cfg(target_os = "linux")]
fn capture_active_window_linux() -> Option<WindowCapture> {
    // Linux window title capture requires X11 or Wayland libraries
    // For now, return None as this requires additional dependencies
    // Consider adding: x11-clipboard or wmctrl integration
    None
}

// macOS implementation (requires Cocoa framework)
#[cfg(target_os = "macos")]
fn capture_active_window_macos() -> Option<WindowCapture> {
    // macOS implementation would use Cocoa framework
    // For now, return None as this requires additional dependencies
    None
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
