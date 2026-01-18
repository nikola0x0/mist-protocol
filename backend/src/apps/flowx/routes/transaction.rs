use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use super::super::config::Config;
use super::super::transaction;

// ============= State Types =============

#[derive(Clone)]
pub struct FlowXState {
    pub sui_client: sui_sdk::SuiClient,
    pub config: Config,
}

// ============= Request/Response Types =============

#[derive(Debug, Deserialize)]
pub struct BuildSwapRequest {
    pub user_address: String,
    pub amount: u64,
    pub min_amount_out: u64,
    pub is_sui_to_token: bool, // true = SUI→MIST, false = MIST→SUI
}

#[derive(Debug, Serialize)]
pub struct BuildSwapResponse {
    pub tx_bytes: String, // Base64 encoded unsigned transaction
    pub swap_info: SwapInfo,
    pub estimated_gas: u64,
}

#[derive(Debug, Serialize)]
pub struct SwapInfo {
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub direction: String, // "SUI→MIST" or "MIST→SUI"
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

/// Build an unsigned swap transaction
pub async fn build_swap_transaction(
    State(state): State<Arc<FlowXState>>,
    Json(request): Json<BuildSwapRequest>,
) -> Result<Json<BuildSwapResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Building FlowX swap: {} {} → {} for user {}",
        request.amount,
        if request.is_sui_to_token { "SUI" } else { "MIST" },
        if request.is_sui_to_token { "MIST" } else { "SUI" },
        request.user_address
    );

    // Validate inputs
    if request.amount == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Amount must be greater than 0".to_string(),
            }),
        ));
    }

    // Build unsigned transaction
    match transaction::build_swap_transaction(
        &state.sui_client,
        &state.config,
        &request.user_address,
        request.amount,
        request.min_amount_out,
        request.is_sui_to_token,
    )
    .await
    {
        Ok(tx_data) => {
            // Serialize transaction to bytes
            let tx_bytes = bcs::to_bytes(&tx_data).unwrap();
            let tx_bytes_b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, tx_bytes);

            info!("FlowX transaction built successfully");

            let direction = if request.is_sui_to_token {
                "SUI→MIST".to_string()
            } else {
                "MIST→SUI".to_string()
            };

            Ok(Json(BuildSwapResponse {
                tx_bytes: tx_bytes_b64,
                swap_info: SwapInfo {
                    amount_in: request.amount,
                    min_amount_out: request.min_amount_out,
                    direction,
                },
                estimated_gas: 50_000_000, // 0.05 SUI
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
    State(state): State<Arc<FlowXState>>,
    Json(request): Json<SubmitSignedRequest>,
) -> Result<Json<SubmitSignedResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Submitting signed FlowX transaction to blockchain");

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
            info!("FlowX transaction submitted: {}", response.digest);
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
