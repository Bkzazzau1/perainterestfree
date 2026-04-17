use crate::user_service::handlers;
use crate::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

/// Router for all user-related endpoints
pub fn user_router() -> Router<AppState> {
    Router::new()
        .route(
            "/user/kyc-profile",
            get(handlers::get_kyc_profile).put(handlers::submit_kyc_profile),
        )
        .route("/user/kyc-status", get(handlers::get_kyc_status))
        .route(
            "/user/display-profile",
            put(handlers::update_display_profile),
        )
        .route(
            "/user/contact-otp/request",
            post(handlers::request_contact_otp),
        )
        .route(
            "/user/contact-otp/verify",
            post(handlers::verify_contact_otp),
        )
        .route("/user/kyc/id-scan", post(handlers::mark_id_scan_complete))
        .route(
            "/user/kyc/face-scan",
            post(handlers::mark_face_scan_complete),
        )
    // REMOVED: .route_layer(...)
}
