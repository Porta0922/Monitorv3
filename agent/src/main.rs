#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod monitoring;
pub mod offline_cache;
pub mod inventory;
pub mod device_id;
pub mod rabbitmq_publisher;
pub mod usb_detection;
pub mod usb_file_copy_detection;
pub mod wifi_detection;
pub mod input_tracking;
pub mod keystroke_tracker;
pub mod process_protection;
pub mod osquery_runner;
pub mod tasks;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Duration};
use device_id::{load_or_create_device_identity, get_device_nickname};
use process_protection::ProcessProtection;
use input_tracking::InputTracker;
use keystroke_tracker::KeystrokeTracker;

#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

#[cfg(windows)]
const SERVICE_NAME: &str = "ActivityMonitor";

#[cfg(windows)]
define_windows_service!(ffi_service_main, my_service_main);

#[cfg(windows)]
fn my_service_main(_arguments: Vec<std::ffi::OsString>) {
    if let Err(e) = run_service() {
        tracing::error!("Service failed: {:?}", e);
    }
}

#[cfg(windows)]
fn run_service() -> Result<(), windows_service::Error> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                let _ = shutdown_tx.blocking_send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    tracing::info!("Service running, starting async agent...");
    
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        if let Err(e) = run_agent(shutdown_rx).await {
            tracing::error!("Agent error: {}", e);
        }
    });

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Attempt to load .env from global config path if it exists
    let env_path = if cfg!(windows) {
        r"C:\ProgramData\ActivityMonitor\.env"
    } else {
        "/etc/activity-monitor/.env"
    };
    dotenvy::from_path(env_path).ok();

    // Initialize logging with both console and file output
    let log_dir = if cfg!(windows) {
        r"C:\ProgramData\ActivityMonitor\logs"
    } else {
        "/var/log/activity-monitor"
    };
    
    let is_session_0 = is_running_in_session_0();
    let log_filename = if is_session_0 { "agent_service.log" } else { "agent_user.log" };
    
    let file_appender = tracing_appender::rolling::daily(log_dir, log_filename);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::{fmt, prelude::*, Registry};
    let subscriber = Registry::default()
        .with(fmt::layer().with_ansi(true))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false));

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    
    // Prune logs older than 7 days to prevent unbounded disk growth
    prune_old_logs(log_dir, if is_session_0 { "agent_service.log" } else { "agent_user.log" }, 7);
    
    let version = env!("CARGO_PKG_VERSION");
    println!("ActivityMonitor Agent v{}", version);
    tracing::info!("Starting ActivityMonitor Agent v{}...", version);

    let _instance_guard = {
        #[cfg(windows)]
        {
            use winapi::um::synchapi::CreateMutexW;
            use winapi::um::errhandlingapi::GetLastError;
            use winapi::shared::winerror::ERROR_ALREADY_EXISTS;
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            let session_id = unsafe {
                let mut sid: u32 = 0;
                if winapi::um::processthreadsapi::ProcessIdToSessionId(winapi::um::processthreadsapi::GetCurrentProcessId(), &mut sid) != 0 {
                    sid
                } else {
                    0
                }
            };

            // Use a session-local mutex name (no 'Global\' prefix) to allow non-admin users
            // to acquire it within their own session namespace.
            let mutex_name = format!("ActivityMonitor_Agent_Session_{}", session_id);
            let wide_name: Vec<u16> = OsStr::new(&mutex_name).encode_wide().chain(std::iter::once(0)).collect();
            
            unsafe {
                let handle = CreateMutexW(std::ptr::null_mut(), 0, wide_name.as_ptr());
                if handle.is_null() {
                    let err = GetLastError();
                    tracing::error!("Failed to create single-instance mutex (Error: {}). Exiting.", err);
                    std::process::exit(1);
                } else if GetLastError() == ERROR_ALREADY_EXISTS {
                    tracing::warn!("Another instance of ActivityMonitor is already running in session {}. Exiting.", session_id);
                    winapi::um::handleapi::CloseHandle(handle);
                    std::process::exit(0);
                }
                handle
            }
        }
        #[cfg(not(windows))]
        {
            // Simple fallback for other platforms
            ()
        }
    };

    #[cfg(windows)]
    {
        // On Windows, try to start as a service dispatcher first.
        // If it fails, we assume we're running in a user session/console.
        if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            tracing::info!("Service dispatcher failed ({}). Falling back to console mode...", e);
            
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to build Tokio runtime: {}", e))?;
                
            let (tx, rx) = mpsc::channel(1);
            
            let tx_clone = tx.clone();
            rt.spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    let _ = tx_clone.send(()).await;
                }
            });

            rt.block_on(async {
                run_agent(rx).await
            }).map_err(|e| format!("Agent execution failed: {}", e))?;
        }
    }

    #[cfg(not(windows))]
    {
        tracing::info!("Running in console mode (Unix)...");
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to build Tokio runtime: {}", e))?;
            
        let (tx, rx) = mpsc::channel(1);
        
        let tx_clone = tx.clone();
        rt.spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                let _ = tx_clone.send(()).await;
            }
        });

        rt.block_on(async {
            run_agent(rx).await
        }).map_err(|e| format!("Agent execution failed: {}", e))?;
    }

    Ok(())
}

async fn run_agent(mut shutdown_rx: mpsc::Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🚀 ActivityMonitor Agent v{} starting...", env!("CARGO_PKG_VERSION"));
    
    let is_session_0 = is_running_in_session_0();
    tracing::info!("Mode: {}", if is_session_0 { "Service (Session 0)" } else { "Interactive User" });

    if !is_session_0 {
        // Small delay to ensure the user session/desktop is fully initialized 
        // before we start capturing window handles and setting hooks.
        tracing::debug!("User session detected, waiting 3s for desktop initialization...");
        sleep(Duration::from_secs(3)).await;
    }

    // Load device identity (or create if new)
    let device_identity = load_or_create_device_identity()?;
    tracing::info!("📱 Device ID: {}", device_identity.device_id);
    tracing::info!("💻 Hostname: {}", device_identity.hostname);
    tracing::info!("🔐 Device auth token enabled: {}", std::env::var("AGENT_AUTH_TOKEN").is_ok());
    
    // Check for nickname
    if let Some(nickname) = get_device_nickname() {
        tracing::info!("📛 Nickname: {}", nickname);
    }
    
    // Initialize offline cache with secure hardware-bound key derivation
    let env_key = std::env::var("AGENT_OFFLINE_CACHE_KEY").ok();
    let encryption_key = offline_cache::resolve_secure_key(env_key.as_deref(), &device_identity.device_id);
    
    let db_name = if is_session_0 { "agent_service_cache.db" } else { "agent_user_cache.db" };
    
    let db_path = if cfg!(windows) {
        std::path::PathBuf::from(r"C:\ProgramData\ActivityMonitor").join(db_name)
    } else {
        std::path::PathBuf::from("/var/lib/activity-monitor").join(db_name)
    };

    let cache = Arc::new(
        offline_cache::OfflineCache::new(db_path.to_str().unwrap_or(db_name), &encryption_key)
            .unwrap_or_else(|_| {
                tracing::warn!("Failed to initialize offline cache at {:?}, continuing with memory cache", db_path);
                offline_cache::OfflineCache::new(":memory:", &encryption_key).unwrap()
            })
    );
    
    tracing::info!("✅ Offline cache initialized");
    
    // Initialize Process Protection (Anti-Kill)
    let protection = ProcessProtection::new(device_identity.device_id.to_string(), true);
    if let Err(e) = protection.init() {
        tracing::warn!("⚠️  Process protection initialization warning: {}", e);
    } else {
        tracing::info!("✅ Process protection enabled");
    }
    
    // Initialize Input Tracking (Keyboard/Mouse Heatmaps)
    let input_tracker = Arc::new(InputTracker::new(device_identity.device_id.to_string(), 19));
    input_tracker.set_screen_resolution(1920, 1080).await;
    tracing::info!("✅ Input activity tracking enabled");

    // Initialize Keystroke Tracking (Idle detection + keystroke counting)
    let keystroke_tracker = Arc::new(KeystrokeTracker::new());
    
    // Initialize platform-specific input listener
    #[cfg(target_os = "windows")]
    {
        use keystroke_tracker::windows_input_listener;
        if let Err(e) = windows_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use keystroke_tracker::linux_input_listener;
        if let Err(e) = linux_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use keystroke_tracker::macos_input_listener;
        if let Err(e) = macos_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    // Initialize RabbitMQ publisher
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/%2F".to_string());
    tracing::info!("🔌 RabbitMQ URL configured for agent: {}", rabbitmq_url);

    let initial_publisher = match rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url).await {
        Ok(conn) => {
            tracing::info!("✅ RabbitMQ connected");
            Some(Arc::new(conn))
        }
        Err(e) => {
            tracing::warn!("⚠️  RabbitMQ connection failed: {}. Running in offline mode.", e);
            None
        }
    };

    let publisher: tasks::SharedPublisher = Arc::new(RwLock::new(initial_publisher));

    // Shared flag: set to true when RabbitMQ (re)connects so stateful monitors
    // like WifiMonitor can force-resend their current state to the new server.
    let wifi_resend_flag = Arc::new(AtomicBool::new(false));

    let auth_token = std::env::var("AGENT_AUTH_TOKEN").unwrap_or_else(|_| "dev-agent-token".to_string());
    let device_id_str = device_identity.device_id.to_string();
    let hostname = device_identity.hostname.clone();
    let mac_address = device_identity.mac_address.clone();
    let envelope_metadata = Arc::new(tasks::EventMetadata::new());

    // Construct TaskContext
    let context = Arc::new(tasks::TaskContext {
        device_id: device_id_str,
        hostname,
        mac_address,
        auth_token,
        publisher,
        cache,
        keystroke_tracker,
        input_tracker,
        envelope_metadata,
        wifi_resend_flag,
    });

    // Spawn all modular background telemetry tasks
    tasks::window_activity::spawn(context.clone());
    tasks::heartbeat::spawn(context.clone());
    tasks::usb_detector::spawn(context.clone());
    tasks::usb_copy::spawn(context.clone());
    tasks::wifi_history::spawn(context.clone());
    tasks::running_apps::spawn(context.clone());
    tasks::inventory::spawn(context.clone());
    tasks::heatmap::spawn(context.clone());
    tasks::resource_logger::spawn(context.clone());
    tasks::security_osquery::spawn(context.clone());

    // Spawn modular support/infrastructure routines
    tasks::support::spawn_reconnector(context.clone(), rabbitmq_url);
    tasks::support::spawn_shutdown_listener(context.clone());
    tasks::support::spawn_retry_synchronizer(context.clone());

    tracing::info!("✅ Agent started successfully");
    tracing::info!("📊 Monitoring: focus-activity (2s) | heartbeat (60s) | open apps (60s) | USB (60s) | USB-copy-detect (60s) | WiFi (120s) | inventory (30d) | input summary (60s)");
    
    // Keep agent running until shutdown signal is received
    shutdown_rx.recv().await;
    tracing::info!("🛑 Shutdown signal received. Stopping agent...");

    Ok(())
}

pub fn is_running_in_session_0() -> bool {
    #[cfg(windows)]
    {
        use winapi::um::processthreadsapi::GetCurrentProcessId;
        use winapi::um::processthreadsapi::ProcessIdToSessionId;

        unsafe {
            let mut session_id: u32 = 0;
            if ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id) != 0 {
                return session_id == 0;
            }
        }
        true // Fallback to true (assume service) if check fails
    }
    #[cfg(not(windows))]
    {
        unsafe { libc::getuid() == 0 }
    }
}

fn prune_old_logs(log_dir: &str, prefix: &str, max_days: i64) {
    if let Ok(entries) = std::fs::read_dir(log_dir) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_days);
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with(prefix) && file_name != prefix {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                let modified_utc: chrono::DateTime<chrono::Utc> = modified.into();
                                if modified_utc < cutoff {
                                    let _ = std::fs::remove_file(entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

