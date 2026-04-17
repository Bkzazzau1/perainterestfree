use crate::AppState;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub reference: Option<String>,
    pub status: Option<String>,
    pub trxref: Option<String>,
    pub tx_ref: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct OkResp {
    pub ok: bool,
}

/// POST /api/v1/hooks/payscribe
/// Receives webhook events from Payscribe.
/// For MVP: accept payload and return 200.
/// Next: verify signature + persist event + idempotency.
#[axum::debug_handler]
pub async fn payscribe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_payload): Json<Value>,
) -> impl IntoResponse {
    // TODO: verify signature (once we know Payscribe header name + algo)
    let _secret = Some(state.payscribe_webhook_secret.clone());

    // Minimal safe logging: do NOT log secrets
    let event_type = headers
        .get("x-event-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "received": true,
            "event_type": event_type,
        })),
    )
}

/// GET /payments/payscribe/callback
/// Browser redirect after payment.
/// For MVP: return JSON summary.
/// Later: redirect to Flutter deep link or web page.
#[axum::debug_handler]
pub async fn payscribe_callback(Query(q): Query<CallbackQuery>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "callback": "received",
            "reference": q.reference,
            "status": q.status,
            "trxref": q.trxref,
            "tx_ref": q.tx_ref,
        })),
    )
}

/// Router for Payscribe-specific endpoints
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/v1/hooks/payscribe", post(payscribe_webhook))
        .route("/payments/payscribe/callback", get(payscribe_callback))
        .with_state(state)
}
