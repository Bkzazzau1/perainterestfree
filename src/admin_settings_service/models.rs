use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct UpdateSettingsPayload {
    pub settings: HashMap<String, String>,
}
