// agent/src/keystroke_tracker.rs
// Enhanced keystroke and idle time tracking

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use chrono::{DateTime, Utc, Duration};

/// Idle detection threshold - User is idle if no activity for N seconds
const IDLE_THRESHOLD_SECONDS: u64 = 300; // 5 minutes

/// Keystroke tracking statistics
#[derive(Debug, Clone, Default)]
pub struct KeystrokeStats {
    pub keystroke_count: u64,
    pub last_keystroke_time: Option<DateTime<Utc>>,
    pub is_idle: bool,
    pub idle_duration_seconds: u64,
}

/// Input activity detector and keystroke tracker
pub struct KeystrokeTracker {
    stats: Arc<Mutex<KeystrokeStats>>,
    last_activity_time: Arc<Mutex<DateTime<Utc>>>,
}

impl KeystrokeTracker {
    /// Create a new keystroke tracker
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            stats: Arc::new(Mutex::new(KeystrokeStats {
                keystroke_count: 0,
                last_keystroke_time: Some(now),
                is_idle: false,
                idle_duration_seconds: 0,
            })),
            last_activity_time: Arc::new(Mutex::new(now)),
        }
    }

    /// Record a keystroke event
    pub async fn record_keystroke(&self) {
        let now = Utc::now();
        let mut stats = self.stats.lock().await;
        
        stats.keystroke_count += 1;
        stats.last_keystroke_time = Some(now);
        stats.is_idle = false;
        stats.idle_duration_seconds = 0;
        
        let mut last_activity = self.last_activity_time.lock().await;
        *last_activity = now;
    }

    /// Record a mouse event (click or movement)
    pub async fn record_mouse_activity(&self) {
        let now = Utc::now();
        let mut stats = self.stats.lock().await;
        
        stats.is_idle = false;
        stats.idle_duration_seconds = 0;
        
        let mut last_activity = self.last_activity_time.lock().await;
        *last_activity = now;
    }

    /// Check idle status based on last activity time
    pub async fn update_idle_status(&self) {
        let now = Utc::now();
        let mut stats = self.stats.lock().await;
        let last_activity = self.last_activity_time.lock().await;
        
        let time_since_last_activity = now.signed_duration_since(*last_activity);
        let seconds_idle = time_since_last_activity.num_seconds() as u64;
        
        if seconds_idle >= IDLE_THRESHOLD_SECONDS {
            stats.is_idle = true;
            stats.idle_duration_seconds = seconds_idle;
        } else {
            stats.is_idle = false;
            stats.idle_duration_seconds = 0;
        }
    }

    /// Get current keystroke statistics
    pub async fn get_stats(&self) -> KeystrokeStats {
        self.stats.lock().await.clone()
    }

    /// Reset keystroke count (typically after uploading stats)
    pub async fn reset_keystroke_count(&self) {
        let mut stats = self.stats.lock().await;
        stats.keystroke_count = 0;
    }

    /// Get time since last activity in seconds
    pub async fn time_since_last_activity(&self) -> u64 {
        let now = Utc::now();
        let last_activity = self.last_activity_time.lock().await;
        let duration = now.signed_duration_since(*last_activity);
        duration.num_seconds().max(0) as u64
    }

    /// Detect if user is currently idle
    pub async fn is_idle(&self) -> bool {
        self.stats.lock().await.is_idle
    }
}

#[cfg(target_os = "windows")]
pub mod windows_input_listener {
    use super::*;
    use std::sync::Arc;
    use winapi::um::winuser::{
        SetWindowsHookExA, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
        KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, CallNextHookEx, HC_ACTION,
        WM_KEYDOWN, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MOUSEMOVE,
    };
    use winapi::um::processthreadsapi::GetCurrentThreadId;
    use std::mem;
    use std::ffi::CStr;

    static mut KEYSTROKE_TRACKER: Option<Arc<KeystrokeTracker>> = None;

    /// Initialize Windows input listener
    pub async fn init_input_listener(tracker: Arc<KeystrokeTracker>) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            KEYSTROKE_TRACKER = Some(tracker);
        }

        // Set up low-level keyboard hook
        unsafe {
            let kb_hook = SetWindowsHookExA(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                std::ptr::null_mut(),
                0,
            );

            if kb_hook.is_null() {
                return Err("Failed to set keyboard hook".into());
            }

            // Set up low-level mouse hook
            let mouse_hook = SetWindowsHookExA(
                WH_MOUSE_LL,
                Some(mouse_hook_proc),
                std::ptr::null_mut(),
                0,
            );

            if mouse_hook.is_null() {
                UnhookWindowsHookEx(kb_hook);
                return Err("Failed to set mouse hook".into());
            }
        }

        Ok(())
    }

    unsafe extern "system" fn keyboard_hook_proc(
        code: i32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if code == HC_ACTION as i32 {
            if let Some(ref tracker) = KEYSTROKE_TRACKER {
                let tracker = tracker.clone();
                tokio::spawn(async move {
                    tracker.record_keystroke().await;
                });
            }
        }

        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    unsafe extern "system" fn mouse_hook_proc(
        code: i32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if code == HC_ACTION as i32 {
            if wparam == WM_LBUTTONDOWN as usize || 
               wparam == WM_RBUTTONDOWN as usize ||
               wparam == WM_MOUSEMOVE as usize {
                if let Some(ref tracker) = KEYSTROKE_TRACKER {
                    let tracker = tracker.clone();
                    tokio::spawn(async move {
                        tracker.record_mouse_activity().await;
                    });
                }
            }
        }

        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}

#[cfg(target_os = "linux")]
pub mod linux_input_listener {
    use super::*;
    use std::sync::Arc;

    /// Initialize Linux input listener (placeholder)
    pub async fn init_input_listener(_tracker: Arc<KeystrokeTracker>) -> Result<(), Box<dyn std::error::Error>> {
        tracing::warn!("Linux input listener not yet implemented. Keystroke tracking disabled.");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub mod macos_input_listener {
    use super::*;
    use std::sync::Arc;

    /// Initialize macOS input listener (placeholder)
    pub async fn init_input_listener(_tracker: Arc<KeystrokeTracker>) -> Result<(), Box<dyn std::error::Error>> {
        tracing::warn!("macOS input listener not yet implemented. Keystroke tracking disabled.");
        Ok(())
    }
}

impl Default for KeystrokeTracker {
    fn default() -> Self {
        Self::new()
    }
}
