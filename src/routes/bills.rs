use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::dto::bills::*;
use crate::AppState;
use serde_json::Value;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/bills/cable/bouquets", get(fetch_bouquets))
        .route("/api/v1/bills/epins", get(get_epins))
        .route("/api/v1/bills/epins/vend", post(vend_epin))
        .route("/api/v1/bills/airtime", post(vend_airtime))
}

#[derive(Deserialize)]
pub struct BouquetsQuery {
    pub service: String, // dstv, gotv, startimes, dstvshowmax...
}

#[axum::debug_handler]
async fn fetch_bouquets(
    State(st): State<AppState>,
    Query(q): Query<BouquetsQuery>,
) -> Result<Json<PayscribeResponse<BouquetsMessage>>, (axum::http::StatusCode, String)> {
    let res: PayscribeResponse<BouquetsMessage> = st
        .payscribe
        .get("bouquets", &[("service", q.service)])
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    Ok(Json(res))
}

#[axum::debug_handler]
async fn get_epins(
    State(st): State<AppState>,
) -> Result<Json<PayscribeResponse<EpinsMessage>>, (axum::http::StatusCode, String)> {
    let res: PayscribeResponse<EpinsMessage> = st
        .payscribe
        .get("epins", &[])
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct VendEpinBody {
    pub qty: i64,
    pub id: String,
    #[serde(default)]
    pub reference: Option<String>, // our field name in Pera
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
}

#[axum::debug_handler]
async fn vend_epin(
    State(st): State<AppState>,
    Json(body): Json<VendEpinBody>,
) -> Result<Json<PayscribeResponse<VendEpinMessage>>, (axum::http::StatusCode, String)> {
    let req = VendEpinRequest {
        qty: body.qty,
        id: body.id,
        ref_: body.reference,
        account: body.account,
        phone: body.phone,
    }
    .into_wire();

    let res: PayscribeResponse<VendEpinMessage> = st
        .payscribe
        .post("epins/vend", &req)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;

    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct AirtimeBody {
    pub network: String,
    pub amount: i64,
    pub phone_number: String,
    pub from: String,
    #[serde(default)]
    pub reference: Option<String>,
}

#[axum::debug_handler]
async fn vend_airtime(
    State(st): State<AppState>,
    Json(body): Json<AirtimeBody>,
) -> Result<Json<PayscribeResponse<Value>>, (axum::http::StatusCode, String)> {
    let reference = body.reference.unwrap_or_else(|| Uuid::new_v4().to_string());
    let payload = serde_json::json!({
        "network": body.network,
        "amount": body.amount,
       "phone_number": body.phone_number,
        "from": body.from,
        "ref": reference,
    });

    let res: PayscribeResponse<Value> = st
        .payscribe
        .post("airtime_to_wallet/vend", &payload)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;

    Ok(Json(res))
}
