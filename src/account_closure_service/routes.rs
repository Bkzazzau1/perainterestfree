use crate::account_closure_service::handlers;
use crate::AppState;
use axum::{routing::post, Router};

pub fn closure_router() -> Router<AppState> {
    Router::new().route(
        "/user/close-account",
        post(handlers::request_account_closure),
    )
}
