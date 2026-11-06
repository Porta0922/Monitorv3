mod telemetry_writer;
mod replay_policy;
mod telemetry_contract;
mod api;
mod auth;
mod config;
mod startup_config;
mod agent_identity;
mod human_sessions;
mod broker_identity;
mod agent_enrollment;
mod rabbitmq_consumer;
mod postgres_db;
pub mod domains;

use std::sync::Arc;
use tokio::task;
use dotenvy::dotenv;

use api::{AppState, create_router};
use auth::AuthManager;
use config::RuntimeConfig;
use postgres_db::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    if !human_sessions::production() { dotenv().ok(); }
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args==["--hash-password-stdin"] {
        use std::io::Read;
        let mut password=String::new();std::io::stdin().take(4097).read_to_string(&mut password)?;
        if password.is_empty()||password.len()>4096{return Err("Password must contain 1..4096 bytes".into());}
        println!("{}",AuthManager::new("").hash_password(&password)?);return Ok(());
    }
    if args==["--migrate"] {
        let path=std::env::var("MIGRATION_DATABASE_URL_FILE").map_err(|_|"MIGRATION_DATABASE_URL_FILE required")?;
        let url=startup_config::read_secret_file(&path,"MIGRATION_DATABASE_URL")?;
        let parsed=reqwest::Url::parse(&url).map_err(|_|"Invalid migration database URL")?;
        if parsed.username()!="monitor_owner" {return Err("Migration job requires monitor_owner".into());}
        Database::migrate_production(&url).await?;
        return Ok(());
    }
    if !args.is_empty() {return Err("Unknown server arguments".into());}
    startup_config::initialize()?;

    // Initialize logging
    tracing_subscriber::fmt::init();

    let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".to_string());

    if jwt_secret == "dev-secret-change-in-production" {
        tracing::warn!("⚠️  JWT_SECRET is using the default value. Set JWT_SECRET env var in production!");
    }
    let rabbitmq_url = std::env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/".to_string());
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/activitymonitor".to_string());
    let runtime_config = RuntimeConfig::from_env();
    crate::auth::configured_accounts()?;

    tracing::info!("ActivityMonitor Server v{} starting...", env!("CARGO_PKG_VERSION"));
    tracing::info!("Server: {}:{}", server_host, server_port);
    tracing::info!("Database connection configured (credentials omitted)");

    // Initialize PostgreSQL connection
    let db = match Database::connect(&database_url).await {
        Ok(db) => {
            tracing::info!("✅ Connected to PostgreSQL");
            db
        }
        Err(e) => {
            tracing::error!("❌ Failed to connect to PostgreSQL: {}", e);
            return Err(e.into());
        }
    };

    broker_identity::spawn(db.clone());
    human_sessions::spawn_cleanup(db.clone());

    // Initialize authentication manager
    let auth_manager = AuthManager::new(&jwt_secret);

    // Create application state
    let app_state = Arc::new(AppState {
        auth: auth_manager,
        db: db.clone(),
        config: runtime_config.clone(),
        rabbitmq_url: rabbitmq_url.clone(),
    });

    // Build router
    let app = create_router(app_state.clone());

    // Start RabbitMQ consumer in background (non-blocking)
    let rabbitmq_url_clone = rabbitmq_url.clone();
    let db_for_consumer = db.clone();

    task::spawn(async move {
        let mut retry_delay_secs = 5u64;
        loop {
            let run_result = rabbitmq_consumer::RabbitMQConsumer::start_consumer(
                &rabbitmq_url_clone,
                db_for_consumer.clone(),
                runtime_config.clone(),
            )
            .await
            .map_err(|e| e.to_string());

            match run_result {
                Ok(_) => {
                    tracing::info!("RabbitMQ consumer stopped gracefully");
                    break;
                }
                Err(err_msg) => {
                    tracing::warn!(
                        "Failed to start/keep RabbitMQ consumer: {}. Retrying in {}s",
                        err_msg,
                        retry_delay_secs
                    );

                    let details = format!("{}; retry_in_seconds={}", err_msg, retry_delay_secs);
                    let _ = db_for_consumer
                        .insert_audit_event(
                            "server/rabbitmq-consumer",
                            "rabbitmq.consumer.error",
                            "rabbitmq",
                            Some(details.as_str()),
                        )
                        .await;

                    tokio::time::sleep(tokio::time::Duration::from_secs(retry_delay_secs)).await;
                    retry_delay_secs = (retry_delay_secs * 2).min(60);
                }
            }
        }
    });

    // Bind to address and start server
    let listen_addr = format!("{}:{}", server_host, server_port);
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;

    tracing::info!("Server listening on http://{}", listen_addr);
    tracing::info!("API Documentation: http://{}/api/docs (coming soon)", listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
