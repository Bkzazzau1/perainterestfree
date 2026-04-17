use crate::auth::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Creates a new Axum Router that bundles all auth-related routes
pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/send-otp", post(handlers::send_otp))
        .route("/auth/verify-otp", post(handlers::verify_otp))
        .route(
            "/auth/password-reset/request",
            post(handlers::request_password_reset),
        )
        .route(
            "/auth/password-reset/confirm",
            post(handlers::confirm_password_reset),
        )
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
        // Merge our new protected routes
        .merge(protected_router())
}

/// Router for all protected endpoints that require authentication
fn protected_router() -> Router<AppState> {
    Router::new().route("/me", get(handlers::get_me))
    // REMOVED: .route_layer(...) - We will apply auth_middleware to this router in main.rs
}
