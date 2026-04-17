use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Matches 'card_item.dart'
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardItem {
    pub id: Uuid, // Our internal ID
    pub kind: String,
    pub network: String,
    pub currency: String,
    pub holder_name: String,
    pub masked_pan: String,
    pub balance_minor: i64,
    pub activated: bool,
    pub frozen: bool,
    pub allow_foreign: bool,
    pub product: String,
}

// Payload for POST /cards/virtual
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVirtualCardPayload {
    pub network: String, // "visa" or "mastercard"
}

// Payload for POST /cards/physical
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPhysicalCardPayload {
    pub network: String,
    pub delivery_type: String, // "pickup" or "home"
    pub full_name: String,
    pub phone: String,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state_region: Option<String>,
}

// Payload for PUT /cards/{id}/toggles
#[derive(Deserialize)]
pub struct CardToggles {
    pub allow_foreign: bool,
}
