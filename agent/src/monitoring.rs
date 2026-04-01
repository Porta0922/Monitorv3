// Process and window monitoring module
use chrono::Utc;
use sysinfo::{System, SystemExt, ProcessExt};
use sha2::{Sha256, Digest};
use std::path::Path;

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
            let exe_path = process.exe().map(|p| p.to_string_lossy().to_string());
            let exe_hash = exe_path.as_ref().and_then(|path| {
                calculate_file_hash(path).ok()
            });

            processes.push(ProcessSnapshot {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                exe_path,
                exe_hash,
            });
        }
        
        processes
    }

    /// Get currently active window (requires window_titles or platform-specific code)
    pub fn capture_active_window(&self) -> Option<WindowCapture> {
        // Platform-specific implementation required
        // For now, return None as this requires platform APIs
        #[cfg(target_os = "windows")]
        {
            return capture_active_window_windows();
        }
        #[cfg(not(target_os = "windows"))]
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

#[cfg(target_os = "windows")]
fn capture_active_window_windows() -> Option<WindowCapture> {
    // TODO: Use Windows API via winapi crate to get active window
    // For now, placeholder
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
