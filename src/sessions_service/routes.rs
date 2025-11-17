use axum::{
    routing::{get, delete},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::sessions_service::handlers;

/// Router for session management
pub fn sessions_router() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(handlers::get_sessions))
        .route("/sessions/:id", delete(handlers::revoke_session))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}