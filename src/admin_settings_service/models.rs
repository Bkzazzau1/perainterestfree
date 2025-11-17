use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct UpdateSettingsPayload {
    pub settings: HashMap<String, String>,
}