//! Transaction API handlers
//!
//! - get_transactions_by_account (paginated, unified for coins, NFTs, and link wallet)
//! - get_transaction_by_digest (single transaction lookup)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::constants::coin;
use crate::db::models::{CoinTransfer, NftTransfer, LinkWalletHistory, UnifiedTransaction, XWalletAccount};
use crate::webhook::handler::AppState;

// ====== Helper Functions ======

/// Collect unique XIDs from from_id and to_id fields
fn collect_unique_ids(items: &[TransactionResponse]) -> Vec<String> {
    items
        .iter()
        .flat_map(|item| vec![item.from_id.clone(), item.to_id.clone()])
        .flatten()
        .filter(|id| !id.starts_with("0x"))  // Filter out addresses, keep only xids
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Lookup handles for XIDs using batch query
async fn lookup_handles(
    pool: &sqlx::PgPool,
    xids: Vec<String>,
) -> HashMap<String, String> {
    XWalletAccount::find_handles_by_xids(pool, &xids)
        .await
        .unwrap_or_default()
}

// ====== Transaction Response Types ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub tx_digest: String,
    pub tx_type: String,
    pub from_id: Option<String>,
    pub to_id: Option<String>,
    pub from_handle: Option<String>,
    pub to_handle: Option<String>,
    // Coin fields
    pub coin_type: Option<String>,
    pub amount: Option<String>,
    pub amount_mist: Option<i64>,
    // NFT fields
    pub nft_object_id: Option<String>,
    pub nft_type: Option<String>,
    pub nft_name: Option<String>,
    // Link wallet fields
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub tweet_id: Option<String>,
    pub timestamp: i64,
    pub created_at: String,
}

impl TransactionResponse {
    fn from_unified(tx: UnifiedTransaction, decimals_map: &HashMap<String, u8>) -> Self {
        let decimals = tx.coin_type
            .as_ref()
            .and_then(|ct| decimals_map.get(ct))
            .copied()
            .unwrap_or(9);

        let amount = tx.amount.map(|a| coin::format_amount(a as u64, decimals));

        Self {
            tx_digest: tx.tx_digest,
            tx_type: tx.tx_type,
            from_id: tx.from_id,
            to_id: tx.to_id,
            from_handle: None,
            to_handle: None,
            coin_type: tx.coin_type,
            amount,
            amount_mist: tx.amount,
            nft_object_id: tx.nft_object_id,
            nft_type: tx.nft_type,
            nft_name: tx.nft_name,
            from_address: tx.from_address,
            to_address: tx.to_address,
            tweet_id: tx.tweet_id,
            timestamp: tx.timestamp,
            created_at: tx.created_at.to_rfc3339(),
        }
    }

    fn from_coin_transfer(tx: CoinTransfer, decimals: u8) -> Self {
        let tx_type = format!("coin_{}", match tx.transfer_type {
            crate::db::models::CoinTransferType::Transfer => "transfer",
            crate::db::models::CoinTransferType::Deposit => "deposit",
            crate::db::models::CoinTransferType::Withdraw => "withdraw",
        });

        Self {
            tx_digest: tx.tx_digest,
            tx_type,
            from_id: tx.from_id,
            to_id: tx.to_id,
            from_handle: None,
            to_handle: None,
            coin_type: Some(tx.coin_type),
            amount: Some(coin::format_amount(tx.amount as u64, decimals)),
            amount_mist: Some(tx.amount),
            nft_object_id: None,
            nft_type: None,
            nft_name: None,
            from_address: None,
            to_address: None,
            tweet_id: tx.tweet_id,
            timestamp: tx.timestamp,
            created_at: tx.created_at.to_rfc3339(),
        }
    }

    fn from_nft_transfer(tx: NftTransfer) -> Self {
        let tx_type = format!("nft_{}", match tx.transfer_type {
            crate::db::models::NftTransferType::Transfer => "transfer",
            crate::db::models::NftTransferType::Deposit => "deposit",
            crate::db::models::NftTransferType::Withdraw => "withdraw",
        });

        Self {
            tx_digest: tx.tx_digest,
            tx_type,
            from_id: tx.from_id,
            to_id: tx.to_id,
            from_handle: None,
            to_handle: None,
            coin_type: None,
            amount: None,
            amount_mist: None,
            nft_object_id: Some(tx.nft_object_id),
            nft_type: tx.nft_type,
            nft_name: tx.nft_name,
            from_address: None,
            to_address: None,
            tweet_id: tx.tweet_id,
            timestamp: tx.timestamp,
            created_at: tx.created_at.to_rfc3339(),
        }
    }

    fn from_link_wallet_history(tx: LinkWalletHistory) -> Self {
        Self {
            tx_digest: tx.tx_digest,
            tx_type: "link_wallet".to_string(),
            from_id: Some(tx.x_user_id.clone()),
            to_id: None,
            from_handle: None,
            to_handle: None,
            coin_type: None,
            amount: None,
            amount_mist: None,
            nft_object_id: None,
            nft_type: None,
            nft_name: None,
            from_address: if tx.from_address.is_empty() { None } else { Some(tx.from_address) },
            to_address: Some(tx.to_address),
            tweet_id: None,
            timestamp: tx.timestamp,
            created_at: tx.created_at.to_rfc3339(),
        }
    }

    fn with_handles(mut self, handles_map: &HashMap<String, String>) -> Self {
        if let Some(ref from_id) = self.from_id {
            if !from_id.starts_with("0x") {
                self.from_handle = handles_map.get(from_id).cloned();
            }
        }
        if let Some(ref to_id) = self.to_id {
            if !to_id.starts_with("0x") {
                self.to_handle = handles_map.get(to_id).cloned();
            }
        }
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedTransactionsResponse {
    pub data: Vec<TransactionResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub total_pages: i64,
}

// ====== Handlers ======

/// Get transaction history by sui_object_id with pagination (SQL-level via UNION)
pub async fn get_transactions_by_account(
    State(state): State<Arc<AppState>>,
    Path(sui_object_id): Path<String>,
    Query(query): Query<TransactionQuery>,
) -> Result<Json<PaginatedTransactionsResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(10).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    // Get x_user_id from sui_object_id
    let account = match XWalletAccount::find_by_sui_object_id(&state.db, &sui_object_id).await {
        Ok(Some(acc)) => acc,
        Ok(None) => {
            return Ok(Json(PaginatedTransactionsResponse {
                data: vec![],
                total: 0,
                page,
                limit,
                total_pages: 0,
            }));
        }
        Err(err) => {
            tracing::error!("Failed to find account: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let x_user_id = &account.x_user_id;

    // Count total using unified count query
    let total = UnifiedTransaction::count_by_x_user_id(&state.db, x_user_id)
        .await
        .unwrap_or(0);

    // Fetch paginated transactions using UNION query
    let transactions = UnifiedTransaction::find_by_x_user_id_paginated(&state.db, x_user_id, limit, offset)
        .await
        .unwrap_or_default();

    // Collect unique coin types for decimals lookup
    let unique_coin_types: Vec<String> = transactions
        .iter()
        .filter_map(|t| t.coin_type.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Get decimals from cache or fetch
    let mut decimals_map: HashMap<String, u8> = if !unique_coin_types.is_empty() {
        state
            .redis
            .get_coin_decimals_batch(&unique_coin_types)
            .await
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Fetch missing decimals
    let missing_coin_types: Vec<&String> = unique_coin_types
        .iter()
        .filter(|ct| !decimals_map.contains_key(*ct))
        .collect();

    if !missing_coin_types.is_empty() {
        let sui_client = crate::clients::sui_client::SuiClient::new(&state.config.sui_rpc_url);
        for coin_type in missing_coin_types {
            let query_coin_type = if coin_type.starts_with("0x") {
                coin_type.clone()
            } else {
                format!("0x{}", coin_type)
            };
            let decimals = match sui_client.get_coin_metadata(&query_coin_type).await {
                Ok(Some(metadata)) => metadata.decimals,
                _ => 9,
            };
            if let Err(e) = state.redis.set_coin_decimals(coin_type, decimals).await {
                tracing::warn!("Failed to cache coin decimals: {:?}", e);
            }
            decimals_map.insert(coin_type.clone(), decimals);
        }
    }

    // Convert to TransactionResponse
    let responses: Vec<TransactionResponse> = transactions
        .into_iter()
        .map(|tx| TransactionResponse::from_unified(tx, &decimals_map))
        .collect();

    // Batch lookup handles
    let unique_xids = collect_unique_ids(&responses);
    let handles_map = lookup_handles(&state.db, unique_xids).await;

    // Add handles to responses
    let data: Vec<TransactionResponse> = responses
        .into_iter()
        .map(|tx| tx.with_handles(&handles_map))
        .collect();

    let total_pages = if total == 0 { 0 } else { (total as f64 / limit as f64).ceil() as i64 };

    Ok(Json(PaginatedTransactionsResponse {
        data,
        total,
        page,
        limit,
        total_pages,
    }))
}

/// Get a single transaction by digest
pub async fn get_transaction_by_digest(
    State(state): State<Arc<AppState>>,
    Path(digest): Path<String>,
) -> Result<Json<TransactionResponse>, StatusCode> {
    // Try to find in coin_transfers first
    if let Ok(Some(tx)) = CoinTransfer::find_by_digest(&state.db, &digest).await {
        let decimals = state
            .redis
            .get_coin_decimals(&tx.coin_type)
            .await
            .unwrap_or(None)
            .unwrap_or(9);
        let mut response = TransactionResponse::from_coin_transfer(tx, decimals);

        let unique_xids = collect_unique_ids(&[response.clone()]);
        let handles_map = lookup_handles(&state.db, unique_xids).await;
        response = response.with_handles(&handles_map);

        return Ok(Json(response));
    }

    // Try nft_transfers
    if let Ok(Some(tx)) = NftTransfer::find_by_digest(&state.db, &digest).await {
        let mut response = TransactionResponse::from_nft_transfer(tx);

        let unique_xids = collect_unique_ids(&[response.clone()]);
        let handles_map = lookup_handles(&state.db, unique_xids).await;
        response = response.with_handles(&handles_map);

        return Ok(Json(response));
    }

    // Try link_wallet_history
    if let Ok(Some(tx)) = LinkWalletHistory::find_by_digest(&state.db, &digest).await {
        let mut response = TransactionResponse::from_link_wallet_history(tx);

        let unique_xids = collect_unique_ids(&[response.clone()]);
        let handles_map = lookup_handles(&state.db, unique_xids).await;
        response = response.with_handles(&handles_map);

        return Ok(Json(response));
    }

    Err(StatusCode::NOT_FOUND)
}
