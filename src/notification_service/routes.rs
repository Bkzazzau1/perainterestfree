use crate::notification_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn notification_router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(handlers::get_notifications))
        .route(
            "/notifications/unread-count",
            get(handlers::get_unread_count),
        )
        .route("/notifications/:id/read", post(handlers::mark_as_read))
}
