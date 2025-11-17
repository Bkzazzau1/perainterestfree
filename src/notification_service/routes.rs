use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::notification_service::handlers;

/// Router for all notification endpoints
pub fn notification_router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(handlers::get_notifications))
        .route("/notifications/unread-count", get(handlers::get_unread_count))
        .route("/notifications/:id/read", post(handlers::mark_as_read))
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}