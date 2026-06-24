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
pub mod config_manager;
pub mod task_supervisor;
pub mod health_reporter;
pub mod command_channel;
pub mod remote_policy;
pub mod discovery;
pub mod web;
#[cfg(windows)]
pub mod ui;
pub mod updater;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use chrono::Timelike;
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

    let is_session_0 = is_running_in_session_0();

    // Initialize logging with both console and file output
    let log_dir = if cfg!(windows) {
        r"C:\ProgramData\ActivityMonitor\logs".to_string()
    } else {
        if is_session_0 {
            "/var/log/activity-monitor".to_string()
        } else {
            if let Ok(home) = std::env::var("HOME") {
                format!("{}/.local/share/activity-monitor/logs", home)
            } else {
                "/tmp/activity-monitor/logs".to_string()
            }
        }
    };
    
    // Ensure the log directory exists
    let _ = std::fs::create_dir_all(&log_dir);
    
    let log_filename = if is_session_0 { "agent_service.log" } else { "agent_user.log" };
    
    let file_appender = tracing_appender::rolling::daily(&log_dir, log_filename);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::{fmt, prelude::*, Registry, filter::EnvFilter};

    let filter = EnvFilter::new("info")
        .add_directive("activity_monitor_agent=trace".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("h2=warn".parse().unwrap())
        .add_directive("lapin=warn".parse().unwrap())
        .add_directive("amq_protocol_tcp=warn".parse().unwrap())
        .add_directive("pinky_swear=warn".parse().unwrap())
        .add_directive("tower=warn".parse().unwrap())
        .add_directive("rustls=warn".parse().unwrap())
        .add_directive("tokio=warn".parse().unwrap())
        .add_directive("want=warn".parse().unwrap())
        .add_directive("mio=warn".parse().unwrap());

    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::layer().with_ansi(true))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false));

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    
    // Register custom panic hook to log all fatal errors to disk
    register_panic_hook();

    #[cfg(windows)]
    {
        // Register application restart with Windows Error Reporting (WER)
        register_windows_restart();
    }
    
    // Prune logs older than 7 days to prevent unbounded disk growth
    prune_old_logs(&log_dir, if is_session_0 { "agent_service.log" } else { "agent_user.log" }, 7);
    
    // Auto-discovery: find agent-config.json and registry overrides
    // This fills env vars that weren't set by .env (env vars take precedence).
    let discovered = discovery::discover();
    discovery::apply_to_env(&discovered);
    
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
        if is_session_0 {
            std::path::PathBuf::from("/var/lib/activity-monitor").join(db_name)
        } else {
            let user_dir = if let Ok(home) = std::env::var("HOME") {
                std::path::PathBuf::from(home).join(".local/share/activity-monitor")
            } else {
                std::path::PathBuf::from("/tmp/activity-monitor")
            };
            user_dir.join(db_name)
        }
    };

    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

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

    // Initialize Web UI state
    let web_state = Arc::new(web::WebState::new(
        hostname.clone(),
        env!("CARGO_PKG_VERSION").to_string(),
        device_id_str.clone(),
    ));
    let events_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // Initialize Configuration Manager
    let config_manager = config_manager::ConfigManager::new();
    tracing::info!("✅ Configuration manager initialized (v{})", config_manager.version());

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
        config_manager: config_manager.clone(),
        events_counter: Some(events_counter.clone()),
    });

    // Create Task Supervisor for lifecycle management
    let supervisor = task_supervisor::TaskSupervisor::new();

    // Track all background telemetry tasks via supervisor
    supervisor.track("window_activity", {
        let ctx = context.clone();
        move || tasks::window_activity::spawn(ctx.clone())
    }).await;

    supervisor.track("heartbeat", {
        let ctx = context.clone();
        move || tasks::heartbeat::spawn(ctx.clone())
    }).await;

    supervisor.track("usb_detector", {
        let ctx = context.clone();
        move || tasks::usb_detector::spawn(ctx.clone())
    }).await;

    supervisor.track("usb_copy", {
        let ctx = context.clone();
        move || tasks::usb_copy::spawn(ctx.clone())
    }).await;

    supervisor.track("wifi_history", {
        let ctx = context.clone();
        move || tasks::wifi_history::spawn(ctx.clone())
    }).await;

    supervisor.track("running_apps", {
        let ctx = context.clone();
        move || tasks::running_apps::spawn(ctx.clone())
    }).await;

    supervisor.track("inventory", {
        let ctx = context.clone();
        move || tasks::inventory::spawn(ctx.clone())
    }).await;

    supervisor.track("heatmap", {
        let ctx = context.clone();
        move || tasks::heatmap::spawn(ctx.clone())
    }).await;

    supervisor.track("resource_logger", {
        let ctx = context.clone();
        move || tasks::resource_logger::spawn(ctx.clone())
    }).await;

    supervisor.track("security_osquery", {
        let ctx = context.clone();
        move || tasks::security_osquery::spawn(ctx.clone())
    }).await;

    // Track support/infrastructure tasks
    supervisor.track("reconnector", {
        let ctx = context.clone();
        let url = rabbitmq_url.clone();
        move || tasks::support::spawn_reconnector(ctx.clone(), url.clone())
    }).await;

    supervisor.track("retry_synchronizer", {
        let ctx = context.clone();
        move || tasks::support::spawn_retry_synchronizer(ctx.clone())
    }).await;

    // Shutdown listener is NOT supervised (it listens until shutdown)
    tasks::support::spawn_shutdown_listener(context.clone());

    // Start supervisor background monitor (auto-restart crashed tasks)
    supervisor.start_monitor();

    // Spawn health reporter, command channel, and remote policy consumer
    health_reporter::spawn_health_reporter(context.clone(), supervisor.clone(), config_manager.clone());
    command_channel::spawn_command_channel(context.clone(), supervisor.clone(), config_manager.clone());
    remote_policy::spawn_remote_policy_consumer(context.clone(), config_manager.clone());

    // Spawn Web UI (always, works in both service and user mode)
    web::spawn_web_server(web_state.clone());

    // Spawn System Tray (Windows user mode only - no exit option)
    #[cfg(windows)]
    if !is_session_0 {
        ui::spawn_tray(web_state.clone());
    }

    // Spawn status updater for Web UI
    {
        let ws = web_state.clone();
        let ec = events_counter.clone();
        let ctx = context.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let connected = ctx.publisher.read().await.is_some();
                ws.connected.store(connected, std::sync::atomic::Ordering::Relaxed);
                ws.events_today.store(ec.load(std::sync::atomic::Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
            }
        });
    }

    // Spawn daily auto-update checker (runs at 9:00 AM every day)
    {
        tokio::spawn(async move {
            use std::time::Duration;

            let now = chrono::Local::now();
            let now_secs = now.time().num_seconds_from_midnight();
            let target = 9 * 3600u32;
            let initial_delay = if now_secs < target {
                target - now_secs
            } else {
                24 * 3600 - now_secs + target
            };

            tracing::info!("[AutoUpdate] First check in {:.1}h", initial_delay as f64 / 3600.0);
            tokio::time::sleep(Duration::from_secs(initial_delay as u64)).await;

            loop {
                tracing::info!("[AutoUpdate] Checking for updates...");

                match crate::updater::check_for_update() {
                    crate::updater::UpdateStatus::UpdateAvailable { version, download_url } => {
                        tracing::info!("[AutoUpdate] Update v{} available, downloading...", version);
                        let temp_dir = std::env::temp_dir();
                        let dest_path = temp_dir.join("am_update.exe");

                        if let Err(e) = crate::updater::download_and_install(&download_url, &dest_path) {
                            tracing::error!("[AutoUpdate] Download failed: {}", e);
                        } else {
                            let service_name = if cfg!(windows) { "ActivityMonitor" } else { "activity-monitor" };
                            match crate::updater::create_update_script(&dest_path, service_name, &version) {
                                Ok(script_path) => {
                                    tracing::info!("[AutoUpdate] Update script created, applying...");
                                    let _ = std::process::Command::new("cmd.exe")
                                        .args(&["/c", "start", "/min", &script_path.to_string_lossy()])
                                        .spawn();
                                    break;
                                }
                                Err(e) => {
                                    tracing::error!("[AutoUpdate] Failed to create update script: {}", e);
                                }
                            }
                        }
                    }
                    crate::updater::UpdateStatus::UpToDate => {
                        tracing::info!("[AutoUpdate] Already up to date (v{})", env!("CARGO_PKG_VERSION"));
                    }
                    crate::updater::UpdateStatus::Error(e) => {
                        tracing::warn!("[AutoUpdate] Check failed: {}", e);
                    }
                }

                tracing::info!("[AutoUpdate] Next check in 24 hours");
                tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
            }
        });
    }

    tracing::info!("✅ Agent started successfully");
    tracing::info!("📊 Monitoring: focus-activity | heartbeat | open apps | USB | USB-copy-detect | WiFi | inventory | input summary | security");
    tracing::info!("🔧 Supervisor active | Health reporter active | Command channel active | Remote policy active");
    
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

fn register_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "no message"
        };
        
        tracing::error!("🚨 FATAL CRASH: Agent panicked at {}: {}", location, message);
        
        // Wait briefly for the non-blocking logging thread to flush the panic message to disk.
        std::thread::sleep(std::time::Duration::from_millis(500));
    }));
}

#[cfg(windows)]
fn register_windows_restart() {
    unsafe {
        // winapi::um::winbase::RegisterApplicationRestart registers the application for restart by Windows Error Reporting (WER).
        // Passing std::ptr::null() restarts the app with its original command line arguments.
        // 0 flags represents default restart settings.
        let hr = winapi::um::winbase::RegisterApplicationRestart(std::ptr::null(), 0);
        if hr == 0 {
            tracing::info!("✅ Application restart registered with Windows Error Reporting (WER)");
        } else {
            tracing::warn!("⚠️ Failed to register application restart with WER (HRESULT: {})", hr);
        }
    }
}


