// agent/src/keystroke_tracker.rs
// Enhanced keystroke and idle time tracking

use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};

/// Idle detection threshold - User is idle if no activity for N seconds
const IDLE_THRESHOLD_SECONDS: u64 = 300; // 5 minutes

/// Keystroke tracking statistics
#[derive(Debug, Clone, Default)]
pub struct KeystrokeStats {
    pub keystroke_count: u64,
    pub mouse_moves_count: u64,
    pub mouse_clicks_count: u64,
    pub last_keystroke_time: Option<DateTime<Utc>>,
    pub is_idle: bool,
    pub idle_duration_seconds: u64,
    pub total_inactive_seconds_today: u64,
    pub last_idle_accumulation_time: Option<DateTime<Utc>>,
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
                mouse_moves_count: 0,
                mouse_clicks_count: 0,
                last_keystroke_time: Some(now),
                is_idle: false,
                idle_duration_seconds: 0,
                total_inactive_seconds_today: 0,
                last_idle_accumulation_time: Some(now),
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

    /// Record mouse movement
    pub async fn record_mouse_movement(&self) {
        let now = Utc::now();
        let mut stats = self.stats.lock().await;

        stats.mouse_moves_count += 1;
        stats.is_idle = false;
        stats.idle_duration_seconds = 0;

        let mut last_activity = self.last_activity_time.lock().await;
        *last_activity = now;
    }

    /// Record mouse click
    pub async fn record_mouse_click(&self) {
        let now = Utc::now();
        let mut stats = self.stats.lock().await;

        stats.mouse_clicks_count += 1;
        stats.is_idle = false;
        stats.idle_duration_seconds = 0;

        let mut last_activity = self.last_activity_time.lock().await;
        *last_activity = now;
    }

    /// Check idle status based on last activity time
    pub async fn update_idle_status(&self) {
        let mut stats = self.stats.lock().await;
        let now = Utc::now();

        // Daily Reset Logic
        use chrono::{Local, Datelike, Timelike};
        let now_local = Local::now();
        
        // If last accumulation was on a different day, reset counters
        if let Some(last_time) = stats.last_idle_accumulation_time {
            let last_local: DateTime<Local> = DateTime::from(last_time);
            if last_local.date_naive() != now_local.date_naive() {
                tracing::info!(
                    "📅 New day detected ({} -> {}). Resetting daily idle counter (was {}s).",
                    last_local.date_naive(),
                    now_local.date_naive(),
                    stats.total_inactive_seconds_today
                );
                stats.total_inactive_seconds_today = 0;
            }
        }

        let mut seconds_idle = if let Some(os_idle_seconds) = platform_idle_seconds() {
            os_idle_seconds
        } else {
            let last_activity = self.last_activity_time.lock().await;
            now.signed_duration_since(*last_activity).num_seconds().max(0) as u64
        };

        // Cap idle duration by seconds elapsed since midnight local time
        // Compute seconds since midnight using hour, minute, second (Timelike trait)
        let time = now_local.time();
        let seconds_since_midnight = (time.hour() as u64) * 3600 + (time.minute() as u64) * 60 + (time.second() as u64);
        if seconds_idle > seconds_since_midnight {
            seconds_idle = seconds_since_midnight;
        }
        
        let was_idle = stats.is_idle;
        if seconds_idle >= IDLE_THRESHOLD_SECONDS {
            stats.is_idle = true;
            stats.idle_duration_seconds = seconds_idle;
            
            // Accumulate total idle time
            if let Some(last_time) = stats.last_idle_accumulation_time {
                let delta = now.signed_duration_since(last_time).num_seconds().max(0) as u64;
                // We only accumulate if we were already considered idle or just became idle
                // To avoid double counting, we use a simple delta since last update call
                stats.total_inactive_seconds_today += delta;
            }
        } else {
            stats.is_idle = false;
            stats.idle_duration_seconds = 0;
        }

        stats.last_idle_accumulation_time = Some(now);
    }

    /// Get current keystroke statistics
    pub async fn get_stats(&self) -> KeystrokeStats {
        self.stats.lock().await.clone()
    }

    /// Reset per-minute counters after summary upload
    pub async fn reset_minute_counters(&self) {
        let mut stats = self.stats.lock().await;
        stats.keystroke_count = 0;
        stats.mouse_moves_count = 0;
        stats.mouse_clicks_count = 0;
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
fn platform_idle_seconds() -> Option<u64> {
    use winapi::um::sysinfoapi::GetTickCount;
    use winapi::um::winuser::{GetLastInputInfo, LASTINPUTINFO};

    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        if GetLastInputInfo(&mut info) == 0 {
            return None;
        }

        let now_ticks = GetTickCount();
        let idle_ms = now_ticks.saturating_sub(info.dwTime);
        Some((idle_ms / 1000) as u64)
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_idle_seconds() -> Option<u64> {
    None
}

#[cfg(target_os = "windows")]
pub mod windows_input_listener {
    use super::*;
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::sync::OnceLock;
    use std::thread;
    use std::time::Duration;
    use tokio::runtime::Handle;
    use winapi::um::winuser::{
        SetWindowsHookExA, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
        CallNextHookEx, HC_ACTION, GetMessageW, TranslateMessage, DispatchMessageW, MSG,
        WM_KEYDOWN, WM_SYSKEYDOWN, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MOUSEMOVE,
    };

    static KEYSTROKE_TRACKER: OnceLock<Arc<KeystrokeTracker>> = OnceLock::new();
    static TOKIO_HANDLE: OnceLock<Handle> = OnceLock::new();

    /// Initialize Windows input listener
    pub async fn init_input_listener(tracker: Arc<KeystrokeTracker>) -> Result<(), Box<dyn std::error::Error>> {
        let runtime_handle = Handle::current();
        let (init_tx, init_rx) = mpsc::channel::<Result<(), String>>();

        thread::spawn(move || {
            unsafe {
                let _ = KEYSTROKE_TRACKER.set(tracker);
                let _ = TOKIO_HANDLE.set(runtime_handle);

                let kb_hook = SetWindowsHookExA(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook_proc),
                    std::ptr::null_mut(),
                    0,
                );

                if kb_hook.is_null() {
                    let _ = init_tx.send(Err("Failed to set keyboard hook".to_string()));
                    return;
                }

                let mouse_hook = SetWindowsHookExA(
                    WH_MOUSE_LL,
                    Some(mouse_hook_proc),
                    std::ptr::null_mut(),
                    0,
                );

                if mouse_hook.is_null() {
                    UnhookWindowsHookEx(kb_hook);
                    let _ = init_tx.send(Err("Failed to set mouse hook".to_string()));
                    return;
                }

                let _ = init_tx.send(Ok(()));

                // Keep this thread alive and pumping messages so low-level hooks keep receiving events.
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                UnhookWindowsHookEx(mouse_hook);
                UnhookWindowsHookEx(kb_hook);
            }
        });

        match init_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(err.into()),
            Err(_) => Err("Timed out initializing Windows input hooks".into()),
        }
    }

    unsafe extern "system" fn keyboard_hook_proc(
        code: i32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if code == HC_ACTION as i32 {
            if wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize {
                if let (Some(tracker), Some(handle)) = (KEYSTROKE_TRACKER.get(), TOKIO_HANDLE.get()) {
                    let tracker = Arc::clone(tracker);
                    let handle = handle.clone();
                    handle.spawn(async move {
                        tracker.record_keystroke().await;
                    });
                }
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
            if let (Some(tracker), Some(handle)) = (KEYSTROKE_TRACKER.get(), TOKIO_HANDLE.get()) {
                let tracker = Arc::clone(tracker);
                let handle = handle.clone();
                match wparam {
                    w if w == WM_MOUSEMOVE as usize => {
                        handle.spawn(async move {
                            tracker.record_mouse_movement().await;
                        });
                    }
                    w if w == WM_LBUTTONDOWN as usize || w == WM_RBUTTONDOWN as usize => {
                        handle.spawn(async move {
                            tracker.record_mouse_click().await;
                        });
                    }
                    _ => {}
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
