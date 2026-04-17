use serde::{Deserialize, Serialize};
use uuid::Uuid;

// This struct matches the data in 'beneficiaries_view.dart'
// We add 'id' for backend management.
#[derive(Serialize, Deserialize, Debug)]
pub struct Beneficiary {
    #[serde(default = "Uuid::new_v4")] // Will be set on read, ignored on create
    pub id: Uuid,
    pub name: String,
    pub channel: String,
    pub provider: String,
    pub account: String,
}

// Separate struct for create/update payloads
// This avoids the client trying to set the 'id'
#[derive(Deserialize, Debug)]
pub struct BeneficiaryPayload {
    pub name: String,
    pub channel: String,
    pub provider: String,
    pub account: String,
}
