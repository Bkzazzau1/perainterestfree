use sqlx::PgPool;
use uuid::Uuid;

/// Internal function for other services to create notifications
pub async fn create_notification(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    body: &str,
) {
    let result = sqlx::query!(
        "INSERT INTO notifications (user_id, title, body) VALUES ($1, $2, $3)",
        user_id,
        title,
        body
    )
    .execute(pool)
    .await;

    if let Err(e) = result {
        // We log the error but don't fail the main transaction
        eprintln!("🔥 Failed to create notification for user {}: {}", user_id, e);
    }
}