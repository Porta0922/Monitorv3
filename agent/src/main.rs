#![windows_subsystem = "windows"]

pub mod runtime;
pub mod native_worker;
pub mod persistent_file;
mod network_identity;
pub mod session_policy;
pub mod session_relay;
pub mod monitoring;
pub mod offline_cache;
mod outbox_drain;
pub mod inventory;
pub mod inventory_snapshot;
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
mod privacy;
pub mod task_supervisor;
pub mod health_reporter;
mod session_health;
mod capture_diagnostics;
pub mod command_channel;
pub mod remote_policy;
pub mod discovery;
pub mod web;
#[cfg(windows)]
pub mod ui;
pub mod updater;
pub mod uninstall;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Duration};
use device_id::{load_or_create_device_identity, get_device_nickname};
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
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = shutdown_tx.try_send(());
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
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
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

    let failed = rt.block_on(async {
        match run_agent(shutdown_rx).await {
            Ok(()) => false,
            Err(error) => { tracing::error!(%error, "Agent failed"); true }
        }
    });

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(if failed { 1 } else { 0 }),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    if std::env::args().nth(1).as_deref()==Some("--internal-x11-input-worker") {
        return keystroke_tracker::linux_input_listener::worker_main();
    }
    if runtime::is_session() && std::env::var_os("AGENT_CONFIG_FILE").is_none() {
        return Err("Session mode requires a dedicated AGENT_CONFIG_FILE".into());
    }
    // Attempt to load .env from global config path if it exists
    let env_path = if cfg!(windows) {
        r"C:\ProgramData\ActivityMonitor\.env"
    } else {
        "/etc/activity-monitor/.env"
    };
    let env_path = std::env::var("AGENT_CONFIG_FILE").unwrap_or_else(|_| env_path.to_string());
    if std::path::Path::new(&env_path).exists() { dotenvy::from_path(&env_path)?; }
    runtime::validate_mode()?;

    let is_session_0 = is_running_in_session_0();

    // Initialize logging with both console and file output
    let log_dir = std::env::var("AGENT_LOG_DIR").unwrap_or_else(|_|
        runtime::data_dir().join("logs").to_string_lossy().into_owned());

    // Ensure the log directory exists
    std::fs::create_dir_all(&log_dir)?;
    
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
        // Recovery belongs to the Windows Service Control Manager.
    }
    
    // Prune logs older than 7 days to prevent unbounded disk growth
    prune_old_logs(&log_dir, if is_session_0 { "agent_service.log" } else { "agent_user.log" }, 7);
    
    // Auto-discovery: find agent-config.json and registry overrides
    // This fills env vars that weren't set by .env (env vars take precedence).
    if !runtime::is_session() {
        let discovered = discovery::discover();
        discovery::apply_to_env(&discovered);
    }
    
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
            if is_session_0 { return Err(Box::new(e)); }
            tracing::info!("Service dispatcher failed ({}). Falling back to console mode...", e);
            
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to build Tokio runtime: {}", e))?;
                
            let (tx, rx) = mpsc::channel(1);
            
            let tx_clone = tx.clone();
            rt.spawn(async move {
                if runtime::shutdown_signal().await.is_ok() {
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
            if runtime::shutdown_signal().await.is_ok() {
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

    #[cfg(unix)]
    let _process_lock = runtime::lock_instance()?;
    let relay = if runtime::is_session() || (is_session_0 && runtime::relay_enabled()) {
        Some(Arc::new(session_relay::SessionRelay::from_env()?))
    } else { None };
    // Only the service/standalone agent owns device enrollment.
    let device_identity = if runtime::is_session() {
        session_relay::load_identity(relay.as_ref().expect("session relay configured")).await?
    } else { load_or_create_device_identity()? };
    tracing::info!("📱 Device ID: {}", device_identity.device_id);
    tracing::info!("💻 Hostname: {}", device_identity.hostname);
    tracing::info!("🔐 Device auth token enabled: {}", std::env::var("AGENT_AUTH_TOKEN").is_ok());
    
    network_identity::initialize(device_identity.device_id).await.map_err(|e|e.to_string())?;

    // Check for nickname
    if let Some(nickname) = get_device_nickname() {
        tracing::info!("📛 Nickname: {}", nickname);
    }
    
    // Initialize offline cache with secure hardware-bound key derivation
    let env_key = std::env::var("AGENT_OFFLINE_CACHE_KEY").ok().or_else(|| {
        if runtime::is_session() { std::env::var("AGENT_SESSION_RELAY_KEY").ok() } else { None }
    });
    let encryption_key = offline_cache::resolve_secure_key(env_key.as_deref(), &device_identity.device_id);
    
    let db_name = if runtime::is_session() { "session_outbox.db" } else if is_session_0 { "agent_service_cache.db" } else { "agent_user_cache.db" };
    
    let db_path = runtime::data_dir().join(db_name);
    std::fs::create_dir_all(runtime::data_dir())?;
    let cache_path = db_path.to_str().ok_or("Cache path is not valid UTF-8")?;
    let cache = Arc::new(offline_cache::OfflineCache::new(cache_path, &encryption_key)?);
    tracing::info!("Persistent offline cache initialized");
    tracing::info!(capabilities = %runtime::capabilities(), "Agent runtime capabilities");

    // Initialize Input Tracking (Keyboard/Mouse Heatmaps)
    let input_tracker = Arc::new(InputTracker::new(device_identity.device_id.to_string(), 19));
    // No fabricated screen dimensions; coordinate capture is explicitly unavailable.
    tracing::info!("✅ Input activity tracking enabled");

    // Initialize Keystroke Tracking (Idle detection + keystroke counting)
    let keystroke_tracker = Arc::new(KeystrokeTracker::new());
    
    #[allow(unused_mut)]
    let mut native_workers: Vec<native_worker::NativeWorker> = Vec::new();
    if runtime::desktop_capture_available() {
    // Initialize platform-specific input listener
    #[cfg(target_os = "windows")]
    {
        use keystroke_tracker::windows_input_listener;
        match windows_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            Ok(worker) => {
                native_workers.push(worker);
                runtime::mark_input_available();
                tracing::info!("Windows input listener started");
            }
            Err(error) => {runtime::set_input_diagnostic(false,"failed",Some(error.to_string()));tracing::warn!(%error, "Failed to initialize Windows input listener");},
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use keystroke_tracker::linux_input_listener;
        match linux_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            Ok(worker)=>native_workers.push(worker),
            Err(error)=>{runtime::set_input_diagnostic(false,"failed",Some(error.to_string()));tracing::warn!(%error,"Linux input worker could not start");}
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use keystroke_tracker::macos_input_listener;
        if let Err(e) = macos_input_listener::init_input_listener(keystroke_tracker.clone()).await {
            tracing::warn!("⚠️  Failed to initialize keystroke tracking: {}", e);
        } else {
            runtime::mark_input_available();
            tracing::info!("✅ Keystroke tracking enabled");
        }
    }
    
    }

    // Initialize RabbitMQ publisher
    let rabbitmq_url = std::env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/%2F".to_string());
    let rabbitmq_url=network_identity::rabbit_url(&rabbitmq_url);
    tracing::info!("RabbitMQ endpoint configured");

    let initial_publisher = if runtime::is_session() { None } else { match rabbitmq_publisher::RabbitMQPublisher::connect(&rabbitmq_url).await {
        Ok(conn) => {
            tracing::info!("✅ RabbitMQ connected");
            Some(Arc::new(conn))
        }
        Err(e) => {
            tracing::warn!("⚠️  RabbitMQ connection failed: {}. Running in offline mode.", e);
            None
        }
    }
    };

    let publisher: tasks::SharedPublisher = Arc::new(RwLock::new(initial_publisher));

    // Shared flag: set to true when RabbitMQ (re)connects so stateful monitors
    // like WifiMonitor can force-resend their current state to the new server.
    let wifi_resend_flag = Arc::new(AtomicBool::new(false));

    let auth_token = if runtime::is_session() { String::new() } else { std::env::var("AGENT_AUTH_TOKEN").unwrap_or_else(|_| "dev-agent-token".to_string()) };
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
    let config_manager = config_manager::ConfigManager::new()?;
    tracing::info!("✅ Configuration manager initialized (v{})", config_manager.version());

    keystroke_tracker.set_idle_threshold(config_manager.get().await.idle_threshold_seconds);

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

    if is_session_0 {
        for (index,relay) in session_relay::SessionRelay::service_relays()?.into_iter().enumerate() {
            let listener=Arc::new(relay.bind().await?);let ctx=context.clone();
            supervisor.track(&format!("session_relay_{index}"),move || {
                let relay=relay.clone();let listener=listener.clone();let ctx=ctx.clone();
                tokio::spawn(async move {relay.serve(listener,ctx).await})
            }).await;
        }
    }

    if runtime::is_session() {
        #[cfg(windows)]
        native_workers.push(ui::spawn_tray(web_state.clone()));
        let relay = relay.expect("session relay configured");
        session_policy::bootstrap(&context, &relay).await?;
        for name in ["window_activity", "running_apps", "heatmap", "session_input"] {
            let ctx = context.clone();
            supervisor.track(name, move || session_policy::spawn_collector(ctx.clone(), name)).await;
        }
        supervisor.track("session_policy", {
            let ctx = context.clone(); let relay = relay.clone();
            move || session_policy::spawn_sync(ctx.clone(), relay.clone())
        }).await;
        supervisor.track("session_health", {
            let ctx=context.clone();let sup=supervisor.clone();let relay=relay.clone();
            move || session_health::spawn(ctx.clone(),sup.clone(),relay.clone())
        }).await;
        supervisor.track("session_sender", {
            let ctx = context.clone();let relay=relay.clone();
            move || session_relay::spawn_sender(ctx.clone(), relay.clone())
        }).await;
        supervisor.start_monitor();
        tracing::info!("Session collector started with service policy");
        shutdown_rx.recv().await;
        supervisor.shutdown_and_wait().await;
        let closed=context.build_event_envelope("running_apps",1,serde_json::json!({"apps":[],"session_closed":true}));
        // Best effort live delivery after collectors stop; persist for later retry
        // if the service is temporarily unreachable. Crashes use stale reporting.
        if !matches!(relay.deliver("running_apps",&closed).await,Ok(true)){let _=context.publish_or_cache("running_apps",closed).await;}

        for worker in native_workers {
            if let Err(error) = worker.shutdown().await { tracing::warn!(%error, "Native shutdown incomplete"); }
        }
        return Ok(());
    }

    // Track all background telemetry tasks via supervisor
    track_if_enabled(&supervisor, &config_manager, "window_activity", {
        let ctx = context.clone();
        move || tasks::window_activity::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "heartbeat", {
        let ctx = context.clone();
        move || tasks::heartbeat::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "usb_detector", {
        let ctx = context.clone();
        move || tasks::usb_detector::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "usb_copy", {
        let ctx = context.clone();
        move || tasks::usb_copy::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "wifi_history", {
        let ctx = context.clone();
        move || tasks::wifi_history::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "running_apps", {
        let ctx = context.clone();
        move || tasks::running_apps::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "inventory", {
        let ctx = context.clone();
        move || tasks::inventory::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "heatmap", {
        let ctx = context.clone();
        move || tasks::heatmap::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "resource_logger", {
        let ctx = context.clone();
        move || tasks::resource_logger::spawn(ctx.clone())
    }).await;

    track_if_enabled(&supervisor, &config_manager, "security_osquery", {
        let ctx = context.clone();
        move || tasks::security_osquery::spawn(ctx.clone())
    }).await;

    // Track support/infrastructure tasks
    if network_identity::production() {
        let ctx=context.clone();
        supervisor.track("network_identity",move || network_identity::spawn(ctx.clone())).await;
    }
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
    // Administrative shutdown is not a security incident.

    // Start supervisor background monitor (auto-restart crashed tasks)
    supervisor.start_monitor();

    // Spawn health reporter, command channel, and remote policy consumer
    supervisor.track("health_reporter", {
        let ctx = context.clone(); let cfg = config_manager.clone();
        let sup = supervisor.clone();
        move || health_reporter::spawn_health_reporter(ctx.clone(), sup.clone(), cfg.clone())
    }).await;
    supervisor.track("command_channel", {
        let ctx = context.clone(); let cfg = config_manager.clone();
        let sup = supervisor.clone();
        move || command_channel::spawn_command_channel(ctx.clone(), sup.clone(), cfg.clone())
    }).await;
    supervisor.track("remote_policy", {
        let ctx = context.clone(); let cfg = config_manager.clone();
        move || remote_policy::spawn_remote_policy_consumer(ctx.clone(), cfg.clone())
    }).await;


    // Spawn Web UI (always, works in both service and user mode)
    supervisor.track("local_web", {
        let state = web_state.clone();
        move || web::spawn_web_server(state.clone())
    }).await;

    // Spawn System Tray (Windows user mode only - no exit option)
    #[cfg(windows)]
    if !is_session_0 {
        native_workers.push(ui::spawn_tray(web_state.clone()));
    }

    let mut background_tasks = tokio::task::JoinSet::new();
    // Read-only local status; contains no tokens, activity text or administrator commands.
    {
        let ws = web_state.clone();
        let ec = events_counter.clone();
        let ctx = context.clone();
        let sup = supervisor.clone();
        background_tasks.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let connected = ctx.publisher.read().await.is_some();
                ws.connected.store(connected, std::sync::atomic::Ordering::Relaxed);
                let events=ec.load(std::sync::atomic::Ordering::Relaxed);
                ws.events_today.store(events,std::sync::atomic::Ordering::Relaxed);
                let pending=ctx.cache.get_unsynced_event_count().await.ok();
                if let Some(count)=pending {ws.cache_pending.store(count as u64,std::sync::atomic::Ordering::Relaxed);}
                let policy=ctx.config_manager.get().await;
                let tasks:Vec<_>=sup.handles().iter().map(|task|serde_json::json!({"name":task.name,"running":task.running,"has_error":task.last_error.is_some(),"restart_count":task.restart_count})).collect();
                if let Ok(mut status)=ws.local_status.lock() {
                    *status=serde_json::json!({"observed_at":chrono::Utc::now(),"publisher_available":connected,
                        "events_since_start":events,"cache_pending":pending,"monitoring_paused":policy.monitoring_paused,
                        "tasks":tasks});
                }
            }
        });
    }

    // Updates are administrator-owned until signed platform packages are available.

    tracing::info!("✅ Agent started successfully");
    tracing::info!("📊 Monitoring: focus-activity | heartbeat | open apps | USB | USB-copy-detect | WiFi | inventory | input summary | security");
    tracing::info!("🔧 Supervisor active | Health reporter active | Command channel active | Remote policy active");
    
    // Keep agent running until shutdown signal is received
    shutdown_rx.recv().await;
    tracing::info!("🛑 Shutdown signal received. Stopping agent...");
    supervisor.shutdown_and_wait().await;
    background_tasks.shutdown().await;
    for worker in native_workers {
        if let Err(error) = worker.shutdown().await { tracing::warn!(%error, "Native shutdown incomplete"); }
    }

    Ok(())
}

pub fn is_running_in_session_0() -> bool {
    match std::env::var("AGENT_MODE").as_deref() {
        Ok("service") => return true,
        Ok("interactive" | "session") => return false,
        _ => {},
    }
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

async fn track_if_enabled<F>(
    supervisor: &Arc<task_supervisor::TaskSupervisor>,
    config_manager: &Arc<config_manager::ConfigManager>,
    name: &str,
    factory: F,
) where
    F: Fn() -> tokio::task::JoinHandle<()> + Send + 'static,
{
    if !runtime::permits_task(name) {
        tracing::info!(monitor = name, "Monitor unavailable in this runtime; omitted");
        return;
    }
    let factory = Arc::new(std::sync::Mutex::new(factory));
    let config = config_manager.clone();
    let task_name = name.to_string();
    supervisor.track(name, move || {
        session_policy::spawn_service_collector(config.clone(), task_name.clone(), factory.clone())
    }).await;
    tracing::info!(monitor = name, "Collector registered with live policy control");
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


