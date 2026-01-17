//! Account API handlers
//!
//! - get_account_by_wallet
//! - get_account_by_owner
//! - get_account_by_twitter_id
//! - get_my_account (secured with access_token)
//! - search_accounts
//! - get_account_balance
//! - get_account_nfts

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::clients::twitter::TwitterOAuth2Client;
use crate::constants::coin;
use crate::db::models::{AccountBalance, AccountNft, XWalletAccount};
use crate::webhook::handler::AppState;

use super::types::{AccountDetailResponse, AccountResponse, BalanceResponse, TokenBalance};

// ====== Search ======

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub accounts: Vec<AccountResponse>,
    pub count: usize,
}

/// Search accounts by Twitter handle, user ID, or Sui address
pub async fn search_accounts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let query = params.q.trim();

    if query.is_empty() {
        return Ok(Json(SearchResponse {
            accounts: vec![],
            count: 0,
        }));
    }

    let accounts = match XWalletAccount::search(&state.db, query).await {
        Ok(accounts) => accounts,
        Err(err) => {
            tracing::error!("Failed to search accounts: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let count = accounts.len();
    let accounts: Vec<AccountResponse> = accounts.into_iter().map(|a| a.into()).collect();

    Ok(Json(SearchResponse { accounts, count }))
}

// ====== Get Account ======

/// Get account by wallet address (owner_address) - basic info only
pub async fn get_account_by_wallet(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<Json<AccountResponse>, StatusCode> {
    match state.account_cache.find_by_owner_address(&address).await {
        Ok(Some(account)) => Ok(Json(account.into())),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(err) => {
            tracing::error!("Failed to query account by wallet: {:?}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get account by wallet address (owner_address) - with balances
pub async fn get_account_by_owner(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<Json<AccountDetailResponse>, StatusCode> {
    let account = match XWalletAccount::find_by_owner_address(&state.db, &address).await {
        Ok(Some(account)) => account,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(err) => {
            tracing::error!("Failed to query account by owner: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let balances = match AccountBalance::find_by_x_user_id(&state.db, &account.x_user_id).await {
        Ok(balances) => balances,
        Err(err) => {
            tracing::error!("Failed to query balances: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(AccountDetailResponse {
        account: account.into(),
        balances: balances
            .into_iter()
            .map(|b| BalanceResponse {
                coin_type: b.coin_type,
                balance: b.balance.to_string(),
            })
            .collect(),
    }))
}

// ====== Internal Helpers ======

/// Fetches account details with balances by Twitter user ID.
///
/// This is the core logic shared by both public and authenticated endpoints.
/// Returns account info and token balances in a single response.
async fn fetch_account_with_balances(
    state: &AppState,
    twitter_user_id: &str,
) -> Result<AccountDetailResponse, (StatusCode, &'static str)> {
    // Fetch account from cache (falls back to DB if not cached)
    let account = state
        .account_cache
        .find_by_x_user_id(twitter_user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error querying account: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?
        .ok_or((StatusCode::NOT_FOUND, "Account not found"))?;

    // Fetch token balances
    let balances = AccountBalance::find_by_x_user_id(&state.db, twitter_user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error querying balances: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        })?;

    Ok(AccountDetailResponse {
        account: account.into(),
        balances: balances
            .into_iter()
            .map(|b| BalanceResponse {
                coin_type: b.coin_type,
                balance: b.balance.to_string(),
            })
            .collect(),
    })
}

/// Extracts Bearer token from Authorization header.
///
/// Expected format: "Bearer <token>"
fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, (StatusCode, &'static str)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

    auth_header
        .strip_prefix("Bearer ")
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid Authorization format. Expected: Bearer <token>"))
}

/// Verifies Twitter access token and returns the authenticated user's ID.
///
/// Calls Twitter API to validate the token and get user info.
async fn verify_twitter_token(state: &AppState, access_token: &str) -> Result<String, (StatusCode, &'static str)> {
    let oauth_client = TwitterOAuth2Client::new(&state.config);

    let twitter_user = oauth_client
        .get_user_info(access_token)
        .await
        .map_err(|e| {
            tracing::warn!("Twitter token verification failed: {:?}", e);
            (StatusCode::UNAUTHORIZED, "Invalid or expired access token")
        })?;

    tracing::info!(
        "Verified token for user: {} (@{})",
        twitter_user.id,
        twitter_user.username
    );

    Ok(twitter_user.id)
}

// ====== Public Endpoints ======

/// Get account by Twitter user ID (PUBLIC).
///
/// Used by: AccountView page (`/account/:twitter_id`)
///
/// Returns account info and balances for any user.
/// This is public since blockchain data is already public.
pub async fn get_account_by_twitter_id(
    State(state): State<Arc<AppState>>,
    Path(twitter_user_id): Path<String>,
) -> Result<Json<AccountDetailResponse>, (StatusCode, &'static str)> {
    let response = fetch_account_with_balances(&state, &twitter_user_id).await?;
    Ok(Json(response))
}

// ====== Authenticated Endpoints ======

/// Get current user's own account (AUTHENTICATED).
///
/// Used by: MyAccount page (`/my-account`)
///
/// Requires: `Authorization: Bearer <twitter_access_token>` header
///
/// This endpoint verifies the caller's identity via Twitter API,
/// ensuring users can only access their own account data.
/// Prevents localStorage manipulation.
pub async fn get_my_account(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AccountDetailResponse>, (StatusCode, &'static str)> {
    // Step 1: Extract token from header
    let access_token = extract_bearer_token(&headers)?;

    // Step 2: Verify token with Twitter API to get real user ID
    let twitter_user_id = verify_twitter_token(&state, access_token).await?;

    // Step 3: Fetch account data for verified user
    let response = fetch_account_with_balances(&state, &twitter_user_id).await?;
    Ok(Json(response))
}

// ====== Balance ======

/// Get account balance by sui_object_id
pub async fn get_account_balance(
    State(state): State<Arc<AppState>>,
    Path(sui_object_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // First get the account to find x_user_id (cached)
    let account = match state.account_cache.find_by_sui_object_id(&sui_object_id).await {
        Ok(Some(acc)) => acc,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(err) => {
            tracing::error!("Failed to find account: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Query all balances from account_balances table
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT coin_type, COALESCE(balance, 0)
        FROM account_balances
        WHERE x_user_id = $1
        "#,
    )
    .bind(&account.x_user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Build token balances list
    let mut balances: Vec<TokenBalance> = Vec::new();
    let mut sui_balance: i64 = 0;

    for (coin_type, balance) in rows {
        let coin_info = coin::get_coin_info(&coin_type);

        if coin_type.ends_with("::sui::SUI") {
            sui_balance = balance;
        }

        let formatted = coin::format_amount(balance as u64, coin_info.decimals);

        balances.push(TokenBalance {
            symbol: coin_info.symbol.to_string(),
            coin_type,
            balance_raw: balance,
            balance_formatted: formatted,
            decimals: coin_info.decimals,
        });
    }

    // Ensure SUI is always first if present
    balances.sort_by(|a, b| {
        if a.symbol == "SUI" {
            std::cmp::Ordering::Less
        } else if b.symbol == "SUI" {
            std::cmp::Ordering::Greater
        } else {
            a.symbol.cmp(&b.symbol)
        }
    });

    Ok(Json(serde_json::json!({
        "balance_mist": sui_balance,
        "balance_sui": coin::format_amount(sui_balance as u64, coin::SUI.decimals),
        "balances": balances,
        "x_user_id": account.x_user_id,
        "sui_object_id": sui_object_id,
    })))
}

// ====== NFTs ======

#[derive(Debug, Serialize)]
pub struct NftResponse {
    pub nft_object_id: String,
    pub nft_type: String,
    pub name: Option<String>,
    pub image_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AccountNftsResponse {
    pub nfts: Vec<NftResponse>,
    pub count: usize,
}

/// Get NFTs for an account by sui_object_id
pub async fn get_account_nfts(
    State(state): State<Arc<AppState>>,
    Path(sui_object_id): Path<String>,
) -> Result<Json<AccountNftsResponse>, StatusCode> {
    let account = match state.account_cache.find_by_sui_object_id(&sui_object_id).await {
        Ok(Some(acc)) => acc,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(err) => {
            tracing::error!("Failed to find account: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let nfts = match AccountNft::find_by_x_user_id(&state.db, &account.x_user_id).await {
        Ok(nfts) => nfts,
        Err(err) => {
            tracing::error!("Failed to query NFTs: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let count = nfts.len();
    let nfts: Vec<NftResponse> = nfts
        .into_iter()
        .map(|n| NftResponse {
            nft_object_id: n.nft_object_id,
            nft_type: n.nft_type,
            name: n.name,
            image_url: n.image_url,
            created_at: n.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(AccountNftsResponse { nfts, count }))
}
