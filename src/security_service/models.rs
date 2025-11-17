use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordPayload {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPinPayload {
    // We require the user's main password to authorize a PIN change
    pub password: String,
    pub new_pin: String, // The 4-digit PIN
}