use axum::{
    routing::{put, post}, // <-- Added 'post'
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::user_service::handlers;

/// Router for all user-related endpoints
pub fn user_router() -> Router<AppState> {
    Router::new()
        // Updated route from /api/v1/user/profile
        .route("/user/kyc-profile", put(handlers::submit_kyc_profile))
        // --- Added ---
        .route("/user/display-profile", put(handlers::update_display_profile))
        // TODO: .route("/user/avatar", post(handlers::update_avatar))
        // -------------
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}