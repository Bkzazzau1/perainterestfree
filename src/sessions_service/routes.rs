use crate::sessions_service::handlers;
use crate::AppState;
use axum::{
    routing::{delete, get},
    Router,
};

pub fn sessions_router() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(handlers::get_sessions))
        .route("/sessions/:id", delete(handlers::revoke_session))
}
