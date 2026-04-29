// agent/src/keystroke_tracker.rs
// Enhanced keystroke and idle time tracking

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicI64, AtomicBool, Ordering};
use chrono::{DateTime, Utc, TimeZone};

/// Idle detection threshold - User is idle if no activity for N seconds
const IDLE_THRESHOLD_SECONDS: u64 = 300; // 5 minutes

/// Keystroke tracking statistics snapshot
#[derive(Debug, Clone, Default)]
pub struct KeystrokeStats {
    pub keystroke_count: u64,
    pub mouse_moves_count: u64,
    pub mouse_clicks_count: u64,
    pub last_keystroke_time: Option<DateTime<Utc>>,
    pub is_idle: bool,
    pub idle_duration_seconds: u64,
    pub total_inactive_seconds_today: u64,
}

/// Input activity detector and keystroke tracker
pub struct KeystrokeTracker {
    keystroke_count: AtomicU64,
    mouse_moves_count: AtomicU64,
    mouse_clicks_count: AtomicU64,
    last_activity_timestamp: AtomicI64, // Unix timestamp
    last_keystroke_timestamp: AtomicI64, // Unix timestamp
    
    // Status tracking (still needs a lock for complex daily accumulation logic)
    status_mutex: tokio::sync::Mutex<TrackerStatus>,
}

struct TrackerStatus {
    is_idle: bool,
    idle_duration_seconds: u64,
    total_inactive_seconds_today: u64,
    last_idle_accumulation_time: DateTime<Utc>,
}

impl KeystrokeTracker {
    /// Create a new keystroke tracker
    pub fn new() -> Self {
        let now = Utc::now();
        let ts = now.timestamp();
        Self {
            keystroke_count: AtomicU64::new(0),
            mouse_moves_count: AtomicU64::new(0),
            mouse_clicks_count: AtomicU64::new(0),
            last_activity_timestamp: AtomicI64::new(ts),
            last_keystroke_timestamp: AtomicI64::new(ts),
            status_mutex: tokio::sync::Mutex::new(TrackerStatus {
                is_idle: false,
                idle_duration_seconds: 0,
                total_inactive_seconds_today: 0,
                last_idle_accumulation_time: now,
            }),
        }
    }

    /// Record a keystroke event (Sync version for hooks)
    pub fn record_keystroke_sync(&self) {
        let now = Utc::now().timestamp();
        self.keystroke_count.fetch_add(1, Ordering::Relaxed);
        self.last_activity_timestamp.store(now, Ordering::Relaxed);
        self.last_keystroke_timestamp.store(now, Ordering::Relaxed);
    }

    /// Record mouse movement (Sync version for hooks)
    pub fn record_mouse_movement_sync(&self) {
        let now = Utc::now().timestamp();
        self.mouse_moves_count.fetch_add(1, Ordering::Relaxed);
        
        // Rate limit activity timestamp updates to once per 500ms to save CPU
        let last = self.last_activity_timestamp.load(Ordering::Relaxed);
        if now > last {
            self.last_activity_timestamp.store(now, Ordering::Relaxed);
        }
    }

    /// Record mouse click (Sync version for hooks)
    pub fn record_mouse_click_sync(&self) {
        let now = Utc::now().timestamp();
        self.mouse_clicks_count.fetch_add(1, Ordering::Relaxed);
        self.last_activity_timestamp.store(now, Ordering::Relaxed);
    }

    // Keep async wrappers for existing callers if needed, but they are not used in hooks anymore
    pub async fn record_keystroke(&self) { self.record_keystroke_sync(); }
    pub async fn record_mouse_movement(&self) { self.record_mouse_movement_sync(); }
    pub async fn record_mouse_click(&self) { self.record_mouse_click_sync(); }

    /// Check idle status based on last activity time
    pub async fn update_idle_status(&self) {
        let mut status = self.status_mutex.lock().await;
        let now = Utc::now();

        // Daily Reset Logic
        use chrono::{Local, Datelike};
        let now_local = Local::now();
        
        let last_local: DateTime<Local> = DateTime::from(status.last_idle_accumulation_time);
        if last_local.date_naive() != now_local.date_naive() {
            tracing::info!(
                "📅 New day detected ({} -> {}). Resetting daily idle counter (was {}s).",
                last_local.date_naive(),
                now_local.date_naive(),
                status.total_inactive_seconds_today
            );
            status.total_inactive_seconds_today = 0;
        }

        let seconds_idle = if let Some(os_idle_seconds) = platform_idle_seconds() {
            os_idle_seconds
        } else {
            let last_ts = self.last_activity_timestamp.load(Ordering::Relaxed);
            (now.timestamp() - last_ts).max(0) as u64
        };

        if seconds_idle >= IDLE_THRESHOLD_SECONDS {
            status.is_idle = true;
            status.idle_duration_seconds = seconds_idle;
            
            let delta = (now.timestamp() - status.last_idle_accumulation_time.timestamp()).max(0) as u64;
            status.total_inactive_seconds_today += delta;
        } else {
            status.is_idle = false;
            status.idle_duration_seconds = 0;
        }

        status.last_idle_accumulation_time = now;
    }

    /// Get current keystroke statistics
    pub async fn get_stats(&self) -> KeystrokeStats {
        let status = self.status_mutex.lock().await;
        let last_ks = self.last_keystroke_timestamp.load(Ordering::Relaxed);
        
        KeystrokeStats {
            keystroke_count: self.keystroke_count.load(Ordering::Relaxed),
            mouse_moves_count: self.mouse_moves_count.load(Ordering::Relaxed),
            mouse_clicks_count: self.mouse_clicks_count.load(Ordering::Relaxed),
            last_keystroke_time: Some(Utc.timestamp_opt(last_ks, 0).unwrap()),
            is_idle: status.is_idle,
            idle_duration_seconds: status.idle_duration_seconds,
            total_inactive_seconds_today: status.total_inactive_seconds_today,
        }
    }

    /// Reset per-minute counters after summary upload
    pub async fn reset_minute_counters(&self) {
        self.keystroke_count.store(0, Ordering::Relaxed);
        self.mouse_moves_count.store(0, Ordering::Relaxed);
        self.mouse_clicks_count.store(0, Ordering::Relaxed);
    }

    /// Get time since last activity in seconds
    pub async fn time_since_last_activity(&self) -> u64 {
        let now = Utc::now().timestamp();
        let last = self.last_activity_timestamp.load(Ordering::Relaxed);
        (now - last).max(0) as u64
    }

    /// Detect if user is currently idle
    pub async fn is_idle(&self) -> bool {
        self.status_mutex.lock().await.is_idle
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
                if let Some(tracker) = KEYSTROKE_TRACKER.get() {
                    tracker.record_keystroke_sync();
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
            if let Some(tracker) = KEYSTROKE_TRACKER.get() {
                match wparam {
                    w if w == WM_MOUSEMOVE as usize => {
                        tracker.record_mouse_movement_sync();
                    }
                    w if w == WM_LBUTTONDOWN as usize || w == WM_RBUTTONDOWN as usize => {
                        tracker.record_mouse_click_sync();
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
    use rdev::{listen, EventType};

    pub async fn init_input_listener(tracker: Arc<KeystrokeTracker>) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("Initializing Linux input listener (rdev)...");
        
        std::thread::spawn(move || {
            tracing::info!("Starting rdev listener thread...");
            if let Err(error) = listen(move |event| {
                match event.event_type {
                    EventType::KeyPress(_) => {
                        tracker.record_keystroke_sync();
                    }
                    EventType::MouseMove { .. } => {
                        tracker.record_mouse_movement_sync();
                    }
                    EventType::ButtonPress(_) => {
                        tracker.record_mouse_click_sync();
                    }
                    _ => {}
                }
            }) {
                tracing::error!("FATAL: Error in Linux input listener: {:?}", error);
            }
        });
        
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
