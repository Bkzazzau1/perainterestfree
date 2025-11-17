use axum::{routing::get, Router};
use sqlx::PgPool;
use std::net::SocketAddr; // <-- Import
use tokio;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
use dotenvy::dotenv;
use std::env;

// --- MODULES ---
mod auth;
pub mod error;
mod user_service;
mod crypto_service;
mod brails_client;
mod provider_service;
mod wallet_service;
mod webhook_service;
mod payment_service;
mod bills_service;
mod islamic_finance_service;
mod card_service;
mod crypto_provider_client;
mod notification_service;
mod convert_service;
mod security_service;
mod beneficiaries_service;
mod sessions_service;
mod account_closure_service;
mod app_data_service;
mod fraud_service;
mod admin_auth_service;
mod user_admin_service;
mod risk_admin_service;
mod admin_settings_service;
mod admin_management_service;
mod admin_analytics_service; // <-- ADDED
mod email_service; // <-- ADDED

// --- APPLICATION STATE ---
#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
    jwt_secret: String,
    crypto_service: crypto_service::CryptoService,
    brails_client: brails_client::BrailsClient,
    brails_webhook_secret: String,
    crypto_provider_client: crypto_provider_client::CryptoProviderClient,
    email_service: email_service::service::EmailService, // <-- ADDED
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env()) // Reads RUST_LOG
        .with(fmt::layer().json())           // Formats logs as JSON
        .init();

    tracing::info!("Starting Pera backend server...");

    // Load .env file
    dotenv().expect("Failed to read .env file");

    // --- ENVIRONMENT VARIABLES ---
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let encryption_key = env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY must be set");
    let brails_webhook_secret = env::var("BRAILS_WEBHOOK_SECRET")
        .expect("BRAILS_WEBHOOK_SECRET must be set");

    if encryption_key.len() != 32 {
        panic!("ENCRYPTION_KEY must be 32 characters long");
    }

    // --- SERVICE INITIALIZATION ---
    let crypto_service = crypto_service::CryptoService::new(&encryption_key);
    let brails_client = brails_client::BrailsClient::new();
    let crypto_provider_client = crypto_provider_client::CryptoProviderClient::new();
    let email_service = email_service::service::EmailService::new(); // <-- ADDED
    
    // --- DATABASE ---
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool");
    
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations ran successfully");

    // --- STATE ---
    let app_state = AppState { 
        db_pool: pool, 
        jwt_secret,
        crypto_service,
        brails_client,
        brails_webhook_secret,
        crypto_provider_client,
        email_service, // <-- ADDED
    };

    // --- ROUTER ---
    let app = Router::new()
        // Public, unauthenticated routes
        .merge(webhook_service::routes::webhook_router())
        .merge(app_data_service::routes::app_data_router())
        .route("/health", get(health_check))
        // All authenticated API routes
        .nest("/api/v1", api_router())
        .with_state(app_state);

    // --- SERVER STARTUP ---
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!(listen_addr = %addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    // This allows Axum to extract client IP addresses
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// Contains all protected and non-webhook API routes
fn api_router() -> Router<AppState> {
    Router::new()
        // Customer Auth
        .merge(auth::routes::auth_router())
        // Admin Auth (unprotected login)
        .merge(admin_auth_service::routes::admin_login_router())
        
        // Admin Protected Routes
        .nest("/admin", 
            admin_auth_service::routes::admin_protected_router()
                // The /stats endpoint is already in this router
                .merge(user_admin_service::routes::user_admin_router())
                .merge(risk_admin_service::routes::risk_admin_router())
                .merge(admin_settings_service::routes::admin_settings_router())
                .merge(admin_management_service::routes::admin_management_router())
                .merge(admin_analytics_service::routes::analytics_router()) // <-- ADDED
        )
        
        // All other customer-facing routes
        .merge(user_service::routes::user_router())
        .merge(provider_service::routes::provider_router())
        .merge(wallet_service::routes::wallet_router())
        .merge(payment_service::routes::payment_router())
        .merge(bills_service::routes::bills_router())
        .merge(islamic_finance_service::routes::islamic_router())
        .merge(card_service::routes::card_router())
        .merge(crypto_service::routes::crypto_router())
        .merge(notification_service::routes::notification_router())
        .merge(convert_service::routes::convert_router())
        .merge(security_service::routes::security_router())
        .merge(beneficiaries_service::routes::beneficiaries_router())
        .merge(sessions_service::routes::sessions_router())
        .merge(account_closure_service::routes::closure_router())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "Pera backend is healthy!"
}
