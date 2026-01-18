use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use super::{CetusService, cetus};
use super::transaction;

// ============= State Types =============

#[derive(Clone)]
pub struct CetusState {
    pub sui_client: sui_sdk::SuiClient,
    pub http_client: reqwest::Client,
    pub config: super::AppConfig,
}

// ============= Request/Response Types =============

#[derive(Debug, Deserialize)]
pub struct PoolInfoRequest {
    pub token_a: String,
    pub token_b: String,
}

#[derive(Debug, Deserialize)]
pub struct BuildSwapRequest {
    pub user_address: String,
    pub token_a: String,
    pub token_b: String,
    pub amount: u64,
    pub slippage: f64, // 0.01 = 1%
    pub a_to_b: bool,
}

#[derive(Debug, Serialize)]
pub struct BuildSwapResponse {
    pub tx_bytes: String, // Base64 encoded unsigned transaction
    pub pool_info: PoolInfo,
    pub estimated_gas: u64,
}

#[derive(Debug, Serialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub symbol: String,
    pub fee_rate: String,
    pub expected_output: u64,
    pub price_impact: f64,
}

#[derive(Debug, Deserialize)]
pub struct SubmitSignedRequest {
    pub signed_tx_bytes: String, // Base64 encoded signed transaction
}

#[derive(Debug, Serialize)]
pub struct SubmitSignedResponse {
    pub digest: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============= API Handlers =============

pub async fn get_pools(
    State(state): State<Arc<CetusState>>,
) -> Result<Json<Vec<cetus::CetusPool>>, (StatusCode, Json<ErrorResponse>)> {
    info!("Fetching all Cetus pools");

    match CetusService::fetch_pools(&state.http_client).await {
        Ok(pools) => {
            info!("Found {} pools", pools.len());
            Ok(Json(pools))
        }
        Err(e) => {
            error!("Failed to fetch pools: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch pools: {}", e),
                }),
            ))
        }
    }
}

pub async fn get_pool_info(
    State(state): State<Arc<CetusState>>,
    Json(request): Json<PoolInfoRequest>,
) -> Result<Json<cetus::CetusPool>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Looking up Cetus pool for {} <-> {}",
        request.token_a, request.token_b
    );

    match CetusService::find_pool(&state.http_client, &request.token_a, &request.token_b).await {
        Ok(Some(pool)) => {
            info!("Found pool: {}", pool.symbol);
            Ok(Json(pool))
        }
        Ok(None) => {
            error!("Pool not found");
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Pool not found for this token pair".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Error finding pool: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to find pool: {}", e),
                }),
            ))
        }
    }
}

/// Build an unsigned swap transaction
pub async fn build_swap_transaction(
    State(state): State<Arc<CetusState>>,
    Json(request): Json<BuildSwapRequest>,
) -> Result<Json<BuildSwapResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Building Cetus swap: {} {} → {} (a_to_b: {}) for user {}",
        request.amount, request.token_a, request.token_b, request.a_to_b, request.user_address
    );

    // Use hardcoded pools (Cetus API is currently unavailable)
    let mut all_pools = vec![
        cetus::CetusPool {
            swap_account: "0x06d8af9e6afd27262db436f0d37b304a041f710c3ea1fa4c3a9bab36b3569ad3".to_string(),
            symbol: "USDT-SUI".to_string(),
            coin_a_address: "0xc060006111016b8a020ad5b33834984a437aaa7d3c74c18e09a95d48aceab08c::coin::COIN".to_string(),
            coin_b_address: "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI".to_string(),
            fee_rate: "0.0025".to_string(),
            current_sqrt_price: "".to_string(),
            tvl_in_usd: "".to_string(),
            vol_in_usd_24h: "".to_string(),
        },
        cetus::CetusPool {
            swap_account: "0x2e041f3fd93646dcc877f783c1f2b7fa62d30271bdef1f21ef002cebf857bded".to_string(),
            symbol: "CETUS-SUI".to_string(),
            coin_a_address: "0x06864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS".to_string(),
            coin_b_address: "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI".to_string(),
            fee_rate: "0.0025".to_string(),
            current_sqrt_price: "".to_string(),
            tvl_in_usd: "".to_string(),
            vol_in_usd_24h: "".to_string(),
        },
        cetus::CetusPool {
            swap_account: "0xcf994611fd4c48e277ce3ffd4d4364c914af2c3cbb05f7bf6facd371de688630".to_string(),
            symbol: "USDC-SUI (Wormhole)".to_string(),
            coin_a_address: "0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::coin::COIN".to_string(),
            coin_b_address: "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI".to_string(),
            fee_rate: "0.0025".to_string(),
            current_sqrt_price: "".to_string(),
            tvl_in_usd: "".to_string(),
            vol_in_usd_24h: "".to_string(),
        },
    ];

    // Add hardcoded USDC-SUI Native pool
    all_pools.push(cetus::CetusPool {
        swap_account: "0x51e883ba7c0b566a26cbc8a94cd33eb0abd418a77cc1e60ad22fd9b1f29cd2ab"
            .to_string(),
        symbol: "USDC-SUI".to_string(),
        coin_a_address:
            "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC"
                .to_string(),
        coin_b_address:
            "0x0000000000000000000000000000000000000000000000000000000000000002::sui::SUI"
                .to_string(),
        fee_rate: "0.0025".to_string(),
        current_sqrt_price: "".to_string(),
        tvl_in_usd: "".to_string(),
        vol_in_usd_24h: "".to_string(),
    });

    // Find pool matching the token pair
    let pool = all_pools
        .iter()
        .find(|p| {
            (p.coin_a_address == request.token_a && p.coin_b_address == request.token_b)
                || (p.coin_a_address == request.token_b && p.coin_b_address == request.token_a)
        })
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!(
                        "No pool found for token pair {} <-> {}",
                        request.token_a, request.token_b
                    ),
                }),
            )
        })?;

    let min_output = 1u64;

    info!(
        "Building swap with min_output={} (slippage protection disabled)",
        min_output
    );

    // Build unsigned transaction using pool_script_v2
    match transaction::build_swap_transaction_v2(
        &state.sui_client,
        &state.config,
        &request.user_address,
        &pool,
        request.amount,
        min_output,
        request.a_to_b,
        &request.token_a,
        &request.token_b,
    )
    .await
    {
        Ok(tx_data) => {
            // Serialize transaction to bytes
            let tx_bytes = bcs::to_bytes(&tx_data).unwrap();
            let tx_bytes_b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, tx_bytes);

            info!("Cetus transaction built successfully");

            Ok(Json(BuildSwapResponse {
                tx_bytes: tx_bytes_b64,
                pool_info: PoolInfo {
                    pool_address: pool.swap_account.clone(),
                    symbol: pool.symbol.clone(),
                    fee_rate: pool.fee_rate.clone(),
                    expected_output: 0,
                    price_impact: 0.0,
                },
                estimated_gas: 1_000_000,
            }))
        }
        Err(e) => {
            error!("Failed to build transaction: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to build transaction: {}", e),
                }),
            ))
        }
    }
}

pub async fn submit_signed_transaction(
    State(state): State<Arc<CetusState>>,
    Json(request): Json<SubmitSignedRequest>,
) -> Result<Json<SubmitSignedResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Submitting signed Cetus transaction to blockchain");

    // Decode the signed transaction
    let signed_tx_bytes = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &request.signed_tx_bytes,
    ) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid base64: {}", e),
                }),
            ))
        }
    };

    // Deserialize the signed transaction
    let signed_tx: sui_sdk::types::transaction::Transaction =
        match bcs::from_bytes(&signed_tx_bytes) {
            Ok(tx) => tx,
            Err(e) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Failed to deserialize transaction: {}", e),
                    }),
                ))
            }
        };

    // Submit to blockchain
    match state.sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            signed_tx,
            sui_sdk::rpc_types::SuiTransactionBlockResponseOptions::full_content(),
            None, // Use default execution strategy
        )
        .await
    {
        Ok(response) => {
            info!("Cetus transaction submitted: {}", response.digest);
            Ok(Json(SubmitSignedResponse {
                digest: response.digest.to_string(),
                status: "submitted".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to submit transaction: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to submit transaction: {}", e),
                }),
            ))
        }
    }
}
