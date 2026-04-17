use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosurePayload {
    pub reason: Option<String>,
    pub pin: String,
}
