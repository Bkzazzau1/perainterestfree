use serde::Serialize;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UnreadCount {
    pub count: i64,
}