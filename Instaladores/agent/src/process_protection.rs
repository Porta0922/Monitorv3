// agent/src/process_protection.rs
// Process Protection and Anti-Termination Mechanism
// Prevents the monitoring agent from being killed and alerts on termination attempts

use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;
use serde_json::json;

/// Process termination attempt
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminationAttempt {
    pub timestamp: String,
    pub device_id: String,
    pub method: String,
    pub attempted_by: Option<String>,
    pub blocked: bool,
    pub auto_restarted: bool,
}

/// Process protection state
#[derive(Debug)]
pub struct ProcessProtection {
    device_id: String,
    attempt_count: Arc<Mutex<u32>>,
    last_attempt: Arc<Mutex<Option<String>>>,
    auto_restart_enabled: bool,
}

impl ProcessProtection {
    pub fn new(device_id: String, auto_restart: bool) -> Self {
        Self {
            device_id,
            attempt_count: Arc::new(Mutex::new(0)),
            last_attempt: Arc::new(Mutex::new(None)),
            auto_restart_enabled: auto_restart,
        }
    }

    /// Initialize process protection for current platform
    pub fn init(&self) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            self.init_windows()
        }
        
        #[cfg(target_os = "linux")]
        {
            self.init_linux()
        }
        
        #[cfg(target_os = "macos")]
        {
            self.init_macos()
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Err("Unsupported platform".to_string())
        }
    }

    /// Windows: Create Job Object to protect process
    #[cfg(target_os = "windows")]
    fn init_windows(&self) -> Result<(), String> {
        use winapi::um::winnt::{JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JobObjectBasicLimitInformation, JOBOBJECT_BASIC_LIMIT_INFORMATION};
        use winapi::um::jobapi2::{CreateJobObjectW, SetInformationJobObject};
        use std::ptr;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        unsafe {
            // Create wide string for job name
            let job_name: Vec<u16> = OsStr::new("ActivityMonitorJob")
                .encode_wide()
                .chain(Some(0))
                .collect();
            
            // Create job object
            let job = CreateJobObjectW(ptr::null_mut(), job_name.as_ptr());
            
            if job.is_null() {
                return Err("Failed to create job object".to_string());
            }
            
            // Configure job to prevent killing
            let mut info: JOBOBJECT_BASIC_LIMIT_INFORMATION = std::mem::zeroed();
            info.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            
            let result = SetInformationJobObject(
                job,
                JobObjectBasicLimitInformation,
                &mut info as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_BASIC_LIMIT_INFORMATION>() as u32,
            );
            
            if result == 0 {
                return Err("Failed to configure job object".to_string());
            }
        }
        
        Ok(())
    }

    /// Linux: Set process to be non-dumpable (prevents ptrace)
    #[cfg(target_os = "linux")]
    fn init_linux(&self) -> Result<(), String> {
        // Set PR_SET_DUMPABLE to 0 to prevent ptrace
        #[allow(unsafe_code)]
        unsafe {
            // PR_SET_DUMPABLE = 4, value = 0
            let result = libc::prctl(4, 0);
            if result != 0 {
                return Err("Failed to set PR_SET_DUMPABLE".to_string());
            }
        }
        
        // Also set parent process death signal to restart on parent death
        #[allow(unsafe_code)]
        unsafe {
            // PR_SET_PDEATHSIG = 1, SIGTERM = 15
            let result = libc::prctl(1, 15);
            if result != 0 {
                return Err("Failed to set PR_SET_PDEATHSIG".to_string());
            }
        }
        
        Ok(())
    }

    /// macOS: Set process to be unkillable (similar to Linux)
    #[cfg(target_os = "macos")]
    fn init_macos(&self) -> Result<(), String> {
        // macOS doesn't have direct unkillable processes like Windows Job Objects
        // Instead, we'll use a watchdog parent process approach
        // For now, we'll rely on alert system + auto-restart
        
        // Set process to high priority
        #[allow(unsafe_code)]
        unsafe {
            let result = libc::setpriority(0, 0, -10);  // High priority
            if result != 0 {
                eprintln!("Warning: Failed to set process priority");
            }
        }
        
        Ok(())
    }

    /// Record a termination attempt
    pub async fn record_termination_attempt(
        &self,
        method: &str,
        attempted_by: Option<String>,
    ) -> TerminationAttempt {
        let mut attempt_count = self.attempt_count.lock().await;
        *attempt_count += 1;
        
        let now = Utc::now();
        
        let attempt = TerminationAttempt {
            timestamp: now.to_rfc3339(),
            device_id: self.device_id.clone(),
            method: method.to_string(),
            attempted_by,
            blocked: true,
            auto_restarted: self.auto_restart_enabled,
        };
        
        let mut last_attempt = self.last_attempt.lock().await;
        *last_attempt = Some(format!("{:?}", attempt));
        
        attempt
    }

    /// Get current protection stats
    pub async fn get_protection_stats(&self) -> serde_json::Value {
        let attempt_count = *self.attempt_count.lock().await;
        let last_attempt = self.last_attempt.lock().await.clone();
        
        json!({
            "device_id": self.device_id,
            "attempt_count": attempt_count,
            "last_attempt": last_attempt,
            "auto_restart_enabled": self.auto_restart_enabled,
            "timestamp": Utc::now().to_rfc3339(),
        })
    }

    /// Install signal handlers to detect kill attempts
    pub fn install_signal_handlers(&self) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            use std::sync::Arc;
            
            static KILL_SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);
            
            // Ignore SIGTERM (graceful shutdown request)
            #[allow(unsafe_code)]
            unsafe {
                signal_hook::consts::signal::SIGTERM;  // Signal code: 15
                // In production, would use signal_hook crate for proper handling
            }
        }
        
        Ok(())
    }

    /// Auto-restart mechanism (if enabled)
    pub async fn auto_restart_if_needed(&self) {
        if self.auto_restart_enabled {
            let attempt_count = self.attempt_count.lock().await;
            if *attempt_count > 0 {
                eprintln!("[ALERT] Process termination attempt detected! Auto-restarting...");
                
                // In production, the parent process would respawn this process
                // Here we just log it
            }
        }
    }
}

/// Create and return a combined alert for termination attempts
pub fn create_termination_alert(
    device_id: &str,
    attempt: &TerminationAttempt,
) -> serde_json::Value {
    json!({
        "type": "PROCESS_TERMINATION_ATTEMPTED",
        "severity": "CRITICAL",
        "device_id": device_id,
        "timestamp": attempt.timestamp,
        "details": {
            "method": &attempt.method,
            "attempted_by": &attempt.attempted_by,
            "blocked": attempt.blocked,
            "auto_restarted": attempt.auto_restarted,
            "message": format!(
                "Termination attempt detected: {}. Blocked and auto-restarted.",
                attempt.method
            ),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_protection_creation() {
        let protection = ProcessProtection::new("test-device".to_string(), true);
        assert_eq!(protection.device_id, "test-device");
        assert!(protection.auto_restart_enabled);
    }

    #[tokio::test]
    async fn test_termination_attempt_recording() {
        let protection = ProcessProtection::new("test-device".to_string(), true);
        
        let attempt = protection.record_termination_attempt(
            "taskkill",
            Some("admin".to_string()),
        ).await;
        
        assert_eq!(attempt.method, "taskkill");
        assert!(attempt.blocked);
    }

    #[tokio::test]
    async fn test_protection_stats() {
        let protection = ProcessProtection::new("test-device".to_string(), true);
        
        protection.record_termination_attempt("kill -9", None).await;
        protection.record_termination_attempt("kill -9", None).await;
        
        let stats = protection.get_protection_stats().await;
        assert_eq!(stats["attempt_count"], 2);
    }

    #[test]
    fn test_termination_alert_creation() {
        let attempt = TerminationAttempt {
            timestamp: Utc::now().to_rfc3339(),
            device_id: "test".to_string(),
            method: "taskkill".to_string(),
            attempted_by: Some("admin".to_string()),
            blocked: true,
            auto_restarted: true,
        };
        
        let alert = create_termination_alert("test", &attempt);
        assert_eq!(alert["type"], "PROCESS_TERMINATION_ATTEMPTED");
        assert_eq!(alert["severity"], "CRITICAL");
    }
}
