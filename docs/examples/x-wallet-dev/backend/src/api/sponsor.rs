//! Transaction sponsorship API handlers (Enoki)
//!
//! - sponsor_transaction
//! - execute_sponsored_transaction

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::clients::enoki::EnokiClient;
use crate::webhook::handler::AppState;

// ====== Sponsor Transaction ======

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SponsorTxRequest {
    pub network: String,
    pub tx_bytes: String,
    pub sender: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub allowed_addresses: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SponsorTxResponse {
    pub bytes: String,
    pub digest: String,
}

#[derive(Debug, Serialize)]
pub struct SponsorErrorResponse {
    pub error: String,
}

/// Create a sponsored transaction using Enoki
pub async fn sponsor_transaction(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SponsorTxRequest>,
) -> Result<Json<SponsorTxResponse>, (StatusCode, Json<SponsorErrorResponse>)> {
    tracing::info!(
        sender = %request.sender,
        network = %request.network,
        "Sponsor transaction request received"
    );

    let enoki_client = EnokiClient::new(
        state.config.enoki_api_key.clone(),
        request.network.clone(),
    );

    match enoki_client
        .create_sponsored_transaction(request.tx_bytes, request.sender)
        .await
    {
        Ok(response) => {
            tracing::info!(digest = %response.digest, "Transaction sponsored successfully");
            Ok(Json(SponsorTxResponse {
                bytes: response.bytes,
                digest: response.digest,
            }))
        }
        Err(err) => {
            tracing::error!("Failed to sponsor transaction: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SponsorErrorResponse {
                    error: format!("Could not create sponsored transaction: {}", err),
                }),
            ))
        }
    }
}

// ====== Execute Sponsored Transaction ======

#[derive(Debug, Deserialize)]
pub struct ExecuteSponsoredTxRequest {
    pub digest: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct ExecuteSponsoredTxResponse {
    pub digest: String,
}

/// Execute a sponsored transaction using Enoki
pub async fn execute_sponsored_transaction(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExecuteSponsoredTxRequest>,
) -> Result<Json<ExecuteSponsoredTxResponse>, (StatusCode, Json<SponsorErrorResponse>)> {
    tracing::info!(
        digest = %request.digest,
        "Execute sponsored transaction request received"
    );

    let enoki_client = EnokiClient::new(
        state.config.enoki_api_key.clone(),
        state.config.enoki_network.clone(),
    );

    match enoki_client
        .execute_sponsored_transaction(request.digest, request.signature)
        .await
    {
        Ok(response) => {
            tracing::info!(digest = %response.digest, "Sponsored transaction executed successfully");
            Ok(Json(ExecuteSponsoredTxResponse {
                digest: response.digest,
            }))
        }
        Err(err) => {
            tracing::error!("Failed to execute sponsored transaction: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SponsorErrorResponse {
                    error: format!("Could not execute sponsored transaction: {}", err),
                }),
            ))
        }
    }
}
