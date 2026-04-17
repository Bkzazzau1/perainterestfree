use axum::{middleware, routing::get, Router};
use dotenvy::dotenv;
use sqlx::PgPool;
use std::env;
use std::net::SocketAddr;
use tokio;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// --- MODULES ---
mod account_closure_service;
mod admin_analytics_service;
mod admin_auth_service;
mod admin_management_service;
mod admin_settings_service;
mod app_data_service;
mod auth;
mod beneficiaries_service;
mod bills_service;
mod brails_client;
mod brails_customer_service;
mod card_service;
mod cash_deposit_service;
mod cash_withdrawal_service;
mod convert_service;
mod crypto_provider_client;
mod crypto_service;
mod dto;
mod email_service;
mod encryption_service;
pub mod error;
mod fraud_service;
mod islamic_finance_service;
mod notification_service;
mod otp_service;
mod payment_service;
mod payscribe_client;
mod provider_service;
mod risk_admin_service;
mod routes;
mod security_service;
mod services;
mod sessions_service;
mod user_admin_service;
mod user_service;
mod wallet_service;
mod webhook_service;

// --- APPLICATION STATE ---
#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
    jwt_secret: String,
    crypto_service: encryption_service::CryptoService,
    brails_client: brails_client::BrailsClient,
    #[allow(dead_code)]
    brails_webhook_secret: String,
    crypto_provider_client: crypto_provider_client::CryptoProviderClient,
    email_service: email_service::service::EmailService,
    payscribe_client: payscribe_client::PayscribeClient,
    #[allow(dead_code)]
    payscribe_webhook_secret: String,
    payscribe: std::sync::Arc<services::payscribe_client::PayscribeClient>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env()) // Reads RUST_LOG
        .with(fmt::layer().json()) // Formats logs as JSON
        .init();

    tracing::info!("Starting Pera backend server...");

    // Load .env file
    dotenv().expect("Failed to read .env file");

    // --- ENVIRONMENT VARIABLES ---
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let encryption_key = env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY must be set");
    let brails_webhook_secret =
        env::var("BRAILS_WEBHOOK_SECRET").expect("BRAILS_WEBHOOK_SECRET must be set");

    let payscribe_base_url =
        env::var("PAYSCRIBE_BASE_URL").expect("PAYSCRIBE_BASE_URL must be set");
    let payscribe_api_key = env::var("PAYSCRIBE_API_KEY").expect("PAYSCRIBE_API_KEY must be set");
    let payscribe_webhook_secret =
        env::var("PAYSCRIBE_WEBHOOK_SECRET").expect("PAYSCRIBE_WEBHOOK_SECRET must be set");

    if encryption_key.len() != 32 {
        panic!("ENCRYPTION_KEY must be 32 characters long");
    }

    // --- SERVICE INITIALIZATION ---
    let crypto_service = encryption_service::CryptoService::new(&encryption_key);
    let brails_client = brails_client::BrailsClient::new();
    let crypto_provider_client = crypto_provider_client::CryptoProviderClient::new();
    let email_service = email_service::service::EmailService::new();
    let payscribe_client =
        payscribe_client::PayscribeClient::new(payscribe_base_url, payscribe_api_key);
    let payscribe = std::sync::Arc::new(services::payscribe_client::PayscribeClient::new(
        std::env::var("PAYSCRIBE_BASE_URL").expect("PAYSCRIBE_BASE_URL must be set"),
        std::env::var("PAYSCRIBE_API_KEY").expect("PAYSCRIBE_API_KEY must be set"),
    ));

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
        email_service,
        payscribe_client,
        payscribe_webhook_secret,
        payscribe,
    };

    // --- ROUTER ---
    let app = Router::new()
        // Public, unauthenticated routes
        .merge(webhook_service::routes::webhook_router())
        .merge(app_data_service::routes::app_data_router())
        .merge(routes::payscribe::router(app_state.clone()))
        .merge(routes::bills::router())
        .route("/health", get(health_check))
        // All API routes (public + protected)
        .nest("/api/v1", api_router(app_state.clone()))
        // Global Axum state (used by handlers via State<AppState>)
        .with_state(app_state);

    // --- SERVER STARTUP ---
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!(listen_addr = %addr, "Server listening");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

/// Contains all protected and non-webhook API routes
fn api_router(app_state: AppState) -> Router<AppState> {
    // 1. Customer-protected routes (JWT auth_middleware)
    let protected_routes = Router::new()
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
        .merge(brails_customer_service::routes::brails_customer_router())
        .merge(cash_withdrawal_service::routes::cash_withdrawal_router())
        .merge(cash_deposit_service::routes::cash_deposit_router())
        .merge(cash_deposit_service::routes::partner_router())
        .merge(cash_withdrawal_service::routes::partner_router())
        // Apply customer auth middleware (with state) to ALL these routes
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::middleware::auth_middleware,
        ));

    // 2. Admin protected routes (with their own admin auth)
    let admin_routes = Router::new().nest(
        "/admin",
        admin_auth_service::routes::admin_protected_router()
            .merge(user_admin_service::routes::user_admin_router())
            .merge(risk_admin_service::routes::risk_admin_router())
            .merge(admin_settings_service::routes::admin_settings_router())
            .merge(admin_management_service::routes::admin_management_router())
            .merge(admin_analytics_service::routes::analytics_router()),
    );

    // 3. Public API (inside /api/v1)
    Router::new()
        // Customer Auth (signup/login, etc.)
        .merge(auth::routes::auth_router())
        // Admin Auth (public admin login)
        .merge(admin_auth_service::routes::admin_login_router())
        // Admin protected under /api/v1/admin/...
        .merge(admin_routes)
        // Customer protected under /api/v1/*
        .merge(protected_routes)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "Pera backend is healthy!"
}
