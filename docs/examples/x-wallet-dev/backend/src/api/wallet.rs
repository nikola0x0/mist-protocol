//! Wallet linking API handlers
//!
//! - generate_link_message
//! - secure_link_wallet

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::clients::enclave::EnclaveClient;
use crate::clients::sui_transaction::SuiTransactionBuilder;
use crate::db::models::XWalletAccount;
use crate::webhook::handler::AppState;

// ====== Generate Link Message ======

#[derive(Debug, Deserialize)]
pub struct GenerateLinkMessageRequest {
    pub xid: String,
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateLinkMessageResponse {
    pub message: String,
    pub timestamp: u64,
}

/// Generate the message that should be signed by the wallet
pub async fn generate_link_message(
    Json(request): Json<GenerateLinkMessageRequest>,
) -> Json<GenerateLinkMessageResponse> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let message = format!(
        "Link XID:{} to wallet {} at {}",
        request.xid, request.wallet_address, timestamp
    );

    Json(GenerateLinkMessageResponse { message, timestamp })
}

// ====== Secure Link Wallet ======

#[derive(Debug, Deserialize)]
pub struct SecureLinkWalletApiRequest {
    pub access_token: String,
    pub wallet_address: String,
    pub wallet_signature: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct LinkWalletResponse {
    pub success: bool,
    pub tx_digest: Option<String>,
    pub error: Option<String>,
}

/// Secure link wallet endpoint
///
/// Flow:
/// 1. Receive request from Dapp with access_token + wallet_signature
/// 2. Forward to Nautilus enclave for verification and signing
/// 3. Submit link_wallet transaction to Sui blockchain
/// 4. Return transaction digest
pub async fn secure_link_wallet(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SecureLinkWalletApiRequest>,
) -> Result<Json<LinkWalletResponse>, StatusCode> {
    tracing::info!(
        "Secure link wallet request for address: {}",
        request.wallet_address
    );

    let enclave_client = EnclaveClient::new(&state.config.enclave_url);

    let signed_payload = match enclave_client
        .sign_secure_link_wallet(
            &request.access_token,
            &request.wallet_address,
            &request.wallet_signature,
            &request.message,
            request.timestamp,
        )
        .await
    {
        Ok(payload) => payload,
        Err(err) => {
            let error_str = err.to_string();
            let error_lower = error_str.to_lowercase();
            tracing::error!("Enclave verification failed: {:?}", err);

            // Check if X access token expired/invalid and return 401 so frontend can refresh token
            // Twitter API returns 401 for expired tokens, 403 for revoked/invalid tokens
            let is_auth_error = error_lower.contains("401")
                || error_lower.contains("unauthorized")
                || error_lower.contains("authentication")
                || error_lower.contains("token expired")
                || error_lower.contains("invalid token");

            if is_auth_error {
                tracing::info!("X access token expired or invalid, returning 401 for token refresh");
                return Err(StatusCode::UNAUTHORIZED);
            }

            return Ok(Json(LinkWalletResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Verification failed: {}", err)),
            }));
        }
    };

    tracing::info!(
        "Enclave signed link wallet for XID: {:?}",
        String::from_utf8_lossy(&signed_payload.response.data.xid)
    );

    let tx_builder = match SuiTransactionBuilder::new(state.config.clone()).await {
        Ok(builder) => builder,
        Err(err) => {
            tracing::error!("Failed to create transaction builder: {:?}", err);
            return Ok(Json(LinkWalletResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Failed to initialize: {}", err)),
            }));
        }
    };

    let xid = String::from_utf8_lossy(&signed_payload.response.data.xid).to_string();

    // Check if wallet is already linked to this account
    if let Ok(Some(account)) = XWalletAccount::find_by_x_user_id(&state.db, &xid).await {
        if let Some(current_wallet) = &account.owner_address {
            if current_wallet.to_lowercase() == request.wallet_address.to_lowercase() {
                tracing::info!("Wallet {} already linked to XID {}", request.wallet_address, xid);
                return Ok(Json(LinkWalletResponse {
                    success: false,
                    tx_digest: None,
                    error: Some("This wallet is already linked to your account".to_string()),
                }));
            }
        }
    }

    match tx_builder
        .link_wallet(
            &xid,
            &request.wallet_address,
            signed_payload.response.timestamp_ms,
            &signed_payload.signature,
        )
        .await
    {
        Ok(digest) => {
            tracing::info!("Link wallet transaction successful: {}", digest);
            Ok(Json(LinkWalletResponse {
                success: true,
                tx_digest: Some(digest),
                error: None,
            }))
        }
        Err(err) => {
            tracing::error!("Link wallet transaction failed: {:?}", err);
            Ok(Json(LinkWalletResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Transaction failed: {}", err)),
            }))
        }
    }
}
