mod api;
mod auth;
mod db;
mod rabbitmq_consumer;
mod whitelist;

use std::sync::Arc;
use tokio::task;
use dotenv::dotenv;

use api::{AppState, create_router};
use auth::AuthManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".to_string());
    let rabbitmq_url = std::env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://guest:guest@localhost:5672/".to_string());
    let database_url = std::env::var("DATABASE_URL").ok();
    
    tracing::info!("ActivityMonitor Server v0.1.0 starting...");
    tracing::info!("Server: {}:{}", server_host, server_port);
    
    // Initialize database connection pool (when ready)
    if let Some(db_url) = &database_url {
        tracing::info!("Connecting to database: {}", db_url.split('@').nth(1).unwrap_or("***"));
        // TODO: Initialize sqlx::PgPool from database_url
        // let pool = sqlx::PgPool::connect(&database_url).await?;
    } else {
        tracing::warn!("DATABASE_URL not set, database features will be unavailable");
    }
    
    // Initialize authentication manager
    let auth_manager = AuthManager::new(&jwt_secret);
    
    // Create application state
    let app_state = Arc::new(AppState {
        auth: auth_manager,
        // db: pool,
    });
    
    // Build router
    let app = create_router(app_state);
    
    // Start RabbitMQ consumer in background (non-blocking)
    let rabbitmq_url_clone = rabbitmq_url.clone();
    task::spawn(async move {
        match rabbitmq_consumer::RabbitMQConsumer::start_consumer(&rabbitmq_url_clone).await {
            Ok(_) => tracing::info!("RabbitMQ consumer started"),
            Err(e) => tracing::warn!("Failed to start RabbitMQ consumer: {}", e),
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
