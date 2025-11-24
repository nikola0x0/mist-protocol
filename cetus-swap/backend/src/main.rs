use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

mod cetus;
mod config;
mod transaction;

use cetus::CetusService;
use config::{AppConfig, Network};

// ============= Application State =============

#[derive(Clone)]
pub struct AppState {
    sui_client: sui_sdk::SuiClient,
    http_client: reqwest::Client,
    config: AppConfig,
}

// ============= Request/Response Types =============

#[derive(Debug, Deserialize)]
pub struct PoolInfoRequest {
    token_a: String,
    token_b: String,
}

#[derive(Debug, Deserialize)]
pub struct BuildSwapRequest {
    user_address: String,
    token_a: String,
    token_b: String,
    amount: u64,
    slippage: f64, // 0.01 = 1%
    a_to_b: bool,
}

#[derive(Debug, Serialize)]
pub struct BuildSwapResponse {
    tx_bytes: String, // Base64 encoded unsigned transaction
    pool_info: PoolInfo,
    estimated_gas: u64,
}

#[derive(Debug, Serialize)]
pub struct PoolInfo {
    pool_address: String,
    symbol: String,
    fee_rate: String,
    expected_output: u64,
    price_impact: f64,
}

#[derive(Debug, Deserialize)]
pub struct SubmitSignedRequest {
    signed_tx_bytes: String, // Base64 encoded signed transaction
}

#[derive(Debug, Serialize)]
pub struct SubmitSignedResponse {
    digest: String,
    status: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

// ============= API Handlers =============

async fn health_check() -> &'static str {
    "Cetus Integration Service is running!"
}

async fn get_pools(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<cetus::CetusPool>>, (StatusCode, Json<ErrorResponse>)> {
    info!("Fetching all pools");

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

async fn get_pool_info(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PoolInfoRequest>,
) -> Result<Json<cetus::CetusPool>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Looking up pool for {} <-> {}",
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
///
/// This endpoint constructs a swap transaction that can be signed by the user's wallet.
/// The transaction is returned as base64-encoded bytes for the frontend to process.
async fn build_swap_transaction(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BuildSwapRequest>,
) -> Result<Json<BuildSwapResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Building swap: {} {} → {} (a_to_b: {}) for user {}",
        request.amount, request.token_a, request.token_b, request.a_to_b, request.user_address
    );

    // Fetch pools from Cetus API and find the one matching the token pair
    let mut all_pools = match cetus::CetusService::fetch_pools(&state.http_client).await {
        Ok(pools) => pools,
        Err(e) => {
            error!("Failed to fetch pools: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to fetch pools: {}", e),
                }),
            ));
        }
    };

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

    // Set minimum output to 1 (essentially disabling slippage protection)
    // TODO: For production, implement proper price calculation by:
    //   1. Querying pool object from blockchain to get current_sqrt_price
    //   2. Using Cetus SDK for accurate price calculation
    //   3. Or using Cetus API's quote endpoint
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

            info!("Transaction built successfully");

            Ok(Json(BuildSwapResponse {
                tx_bytes: tx_bytes_b64,
                pool_info: PoolInfo {
                    pool_address: pool.swap_account.clone(),
                    symbol: pool.symbol.clone(),
                    fee_rate: pool.fee_rate.clone(),
                    expected_output: 0, // Would need real-time pool query to calculate
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

async fn submit_signed_transaction(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SubmitSignedRequest>,
) -> Result<Json<SubmitSignedResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Submitting signed transaction to blockchain");

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
            Some(sui_sdk::types::quorum_driver_types::ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await
    {
        Ok(response) => {
            info!("Transaction submitted: {}", response.digest);
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

// ============= Main Application =============

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Load configuration
    let network = std::env::var("NETWORK")
        .unwrap_or("mainnet".to_string())
        .parse::<Network>()
        .unwrap_or(Network::Mainnet);

    let config = AppConfig::new(network);

    info!(
        "Starting Cetus Integration Service on {:?} network",
        config.network
    );

    // Initialize Sui client
    let sui_client = match config.network {
        Network::Mainnet => sui_sdk::SuiClientBuilder::default().build_mainnet().await?,
        Network::Testnet => sui_sdk::SuiClientBuilder::default().build_testnet().await?,
    };

    info!("Connected to Sui RPC: {}", config.rpc_url);

    let http_client = reqwest::Client::new();

    let app_state = Arc::new(AppState {
        sui_client,
        http_client,
        config,
    });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/pools", get(get_pools))
        .route("/api/pool/info", post(get_pool_info))
        .route("/api/build-swap", post(build_swap_transaction))
        .route("/api/submit-signed", post(submit_signed_transaction))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Start server
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("🚀 Server running on http://{}", addr);
    info!("📖 API Documentation:");
    info!("   GET  /health            - Health check");
    info!("   GET  /api/pools         - Get all pools");
    info!("   POST /api/pool/info     - Get pool info");
    info!("   POST /api/build-swap    - Build swap transaction");
    info!("   POST /api/submit-signed - Submit signed transaction");

    axum::serve(listener, app).await?;

    Ok(())
}
