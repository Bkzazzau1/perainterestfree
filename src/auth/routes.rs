use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::auth::handlers;
use crate::auth::middleware::auth_middleware;
use crate::AppState;

/// Creates a new Axum Router that bundles all auth-related routes
pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(handlers::register))
        .route("/api/v1/auth/send-otp", post(handlers::send_otp))
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/logout", post(handlers::logout)) // <-- Added
        // Merge our new protected routes
        .merge(protected_router()) 
}

/// Router for all protected endpoints that require authentication
fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/me", get(handlers::get_me))
        // Add more protected routes here (e.g., /wallets, /cards)
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}