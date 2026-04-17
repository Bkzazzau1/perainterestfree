use crate::convert_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn convert_router() -> Router<AppState> {
    Router::new()
        .route(
            "/convert/quote",
            get(crate::crypto_service::handlers::get_quote),
        )
        .route("/convert/execute", post(handlers::execute_conversion))
}
