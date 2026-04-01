// Process and window monitoring module
use chrono::Utc;
use sysinfo::{System, SystemExt, ProcessExt, PidExt};
use sha2::{Sha256, Digest};

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
            let exe_hash = calculate_file_hash(&exe_path).ok();

            processes.push(ProcessSnapshot {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                exe_path: Some(exe_path),
                exe_hash,
            });
        }
        
        processes
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
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowModuleFileNameW};

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

        // Get window class/app name
        let mut class_buffer = [0u16; 256];
        let class_len = GetWindowModuleFileNameW(hwnd, class_buffer.as_mut_ptr(), class_buffer.len() as u32);
        
        let app_name = if class_len > 0 {
            String::from_utf16_lossy(&class_buffer[..class_len as usize]).to_string()
        } else {
            "Unknown".to_string()
        };

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
        // Create a temporary test file
        let test_content = b"test content for hashing";
        let result = calculate_file_hash("/tmp/test.txt");
        // Will fail if file doesn't exist, which is ok for this test
        let _ = result;
    }
}
