use axum::{
    extract::{Query, State}, // <-- Added Query
    http::StatusCode, 
    response::IntoResponse, 
    Json, Extension
};
use crate::{error::AppError, AppState};
use crate::auth::jwt::Claims;
// --- UPDATED IMPORT ---
use crate::wallet_service::models::{WalletSummary, Transaction, HistoryQuery};
// ----------------------
use serde_json::json;
use uuid::Uuid;

// The default wallets to create for a new user
const DEFAULT_CURRENCIES: [&str; 5] = ["NGN", "USD", "GHS", "KES", "UGX"];

/// Helper to lazy-provision wallets for a user
async fn provision_wallets(
    pool: &sqlx::PgPool, 
    user_id: Uuid
) -> Result<(), AppError> {
    for currency in DEFAULT_CURRENCIES.iter() {
        sqlx::query!(
            r#"
            INSERT INTO wallets (user_id, currency, balance_minor)
            VALUES ($1, $2, 0)
            ON CONFLICT (user_id, currency) DO NOTHING
            "#,
            user_id,
            currency
        )
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;
    }
    Ok(())
}

/// Handler for GET /api/v1/wallets/summary
/// Returns all wallet balances (for home screen)
pub async fn get_wallet_summary(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {

    let user_id = claims.sub;

    // 1. Ensure all default wallets exist (lazy-provisioning)
    provision_wallets(&state.db_pool, user_id).await?;

    // 2. Fetch all wallets
    let wallets = sqlx::query!(
        r#"SELECT currency, balance_minor FROM wallets WHERE user_id = $1"#,
        user_id
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    // 3. Fetch the NGN virtual account (if it exists)
    let va = sqlx::query!(
        "SELECT account_number FROM virtual_accounts WHERE user_id = $1 AND currency = 'NGN'",
        user_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DatabaseError)?;

    let va_account_number = va.map_or("".to_string(), |r| r.account_number);

    // 4. Build the 'WalletSummary' response
    let wallet_summaries: Vec<WalletSummary> = wallets.into_iter().map(|w| {
        let account_number = if w.currency == "NGN" {
            va_account_number.clone()
        } else {
            "".to_string() // Other currencies are virtual balances
        };
        WalletSummary {
            name: format!("{} Wallet", w.currency),
            account_number,
            currency: w.currency,
            balance_minor: w.balance_minor.unwrap_or(0),
        }
    }).collect();

    // Match the JSON structure from 'wallets_controller.dart'
    Ok((StatusCode::OK, Json(json!({ "wallets": wallet_summaries }))))
}

/// Handler for GET /api/v1/wallets/transactions
/// Returns searchable, paginated transaction history
pub async fn get_transactions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<HistoryQuery>, // <-- Accept query params
) -> Result<impl IntoResponse, AppError> {

    let user_id = claims.sub;
    
    // --- Build dynamic query ---
    let mut sql = "
        SELECT
            id,
            title,
            amount_minor,
            created_at as \"at!\",
            currency,
            status,
            counterparty,
            reference,
            type as transaction_type
        FROM transactions
        WHERE user_id = $1
    ".to_string();

    let mut bind_index = 2;
    
    // Add search 'q' parameter
    let search_term = query.q.map(|s| format!("%{}%", s.to_lowercase()));
    if search_term.is_some() {
        sql.push_str(&format!(
            " AND (LOWER(title) LIKE ${} OR LOWER(counterparty) LIKE ${} OR LOWER(reference) LIKE ${})",
            bind_index, bind_index, bind_index
        ));
        bind_index += 1;
    }
    
    // Add date range
    if query.start_date.is_some() {
        sql.push_str(&format!(" AND created_at >= ${}", bind_index));
        bind_index += 1;
    }
    if query.end_date.is_some() {
        sql.push_str(&format!(" AND created_at <= ${}", bind_index));
        bind_index += 1;
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT 50"); // Add pagination
    
    // --- Bind parameters and execute ---
    let mut query_builder = sqlx::query_as::<_, Transaction>(&sql)
        .bind(user_id);
    
    if let Some(ref term) = search_term {
        query_builder = query_builder.bind(term);
    }
    if let Some(ref start) = query.start_date {
        query_builder = query_builder.bind(start);
    }
    if let Some(ref end) = query.end_date {
        query_builder = query_builder.bind(end);
    }

    let transactions = query_builder
        .fetch_all(&state.db_pool)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok((StatusCode::OK, Json(transactions)))
}