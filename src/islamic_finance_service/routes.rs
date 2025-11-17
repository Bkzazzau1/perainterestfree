use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use crate::auth::middleware::auth_middleware;
use crate::AppState;
use crate::islamic_finance_service::handlers;

/// Router for all Islamic finance endpoints
pub fn islamic_router() -> Router<AppState> {
    Router::new()
        // Zakat
        .route("/islamic/zakat-rates", get(handlers::get_zakat_rates))
        .route("/islamic/pay-zakat", post(handlers::pay_zakat))
        
        // TODO: Umrah endpoints
        // .route("/islamic/umrah-agencies", get(handlers::get_umrah_agencies))
        // .route("/islamic/pay-umrah", post(handlers::pay_umrah))
        
        .route_layer(middleware::from_fn_with_state(
            auth_middleware,
        ))
}