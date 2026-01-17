//! Authentication API handlers
//!
//! - exchange_twitter_token (OAuth 2.0)
//! - create_account

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::clients::enclave::EnclaveClient;
use crate::clients::sui_transaction::SuiTransactionBuilder;
use crate::clients::twitter::{TwitterOAuth2Client, TwitterUserInfo};
use crate::db::models::XWalletAccount;
use crate::webhook::handler::AppState;

// ====== OAuth 2.0 Token Exchange ======

#[derive(Debug, Deserialize)]
pub struct TokenExchangeRequest {
    pub code: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize)]
pub struct XWalletAccountInfo {
    pub sui_object_id: String,
    pub x_user_id: String,
    pub x_handle: String,
    pub owner_address: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: TwitterUserInfo,
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: Option<String>,
    #[serde(rename = "xwalletAccount")]
    pub xwallet_account: Option<XWalletAccountInfo>,
}

#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub error: String,
}

/// Exchange OAuth code for access token and get user info
pub async fn exchange_twitter_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TokenExchangeRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    tracing::info!("Token exchange request received");

    let oauth2_client = TwitterOAuth2Client::new(&state.config);

    let token_response = oauth2_client
        .exchange_code(&request.code, &request.code_verifier, &request.redirect_uri)
        .await
        .map_err(|err| {
            tracing::error!("Token exchange failed: {:?}", err);
            (
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: format!("Token exchange failed: {}", err),
                }),
            )
        })?;

    let user_info = oauth2_client
        .get_user_info(&token_response.access_token)
        .await
        .map_err(|err| {
            tracing::error!("Failed to get user info: {:?}", err);
            (
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: format!("Failed to get user info: {}", err),
                }),
            )
        })?;

    tracing::info!(
        x_user_id = %user_info.id,
        username = %user_info.username,
        "User authenticated successfully"
    );

    let xwallet_account = XWalletAccount::find_by_x_user_id(&state.db, &user_info.id)
        .await
        .map_err(|err| {
            tracing::error!("Database error looking up account: {:?}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "Database error".to_string(),
                }),
            )
        })?;

    if let Some(ref _account) = xwallet_account {
        tracing::info!(x_user_id = %user_info.id, "Found existing XWallet account");

        // Update avatar in DB on login
        if let Err(err) = XWalletAccount::update_avatar(
            &state.db,
            &user_info.id,
            user_info.profile_image_url.as_deref(),
        ).await {
            tracing::warn!("Failed to update avatar: {:?}", err);
        }
    } else {
        tracing::info!(
            x_user_id = %user_info.id,
            "No XWallet account found - user needs to create one"
        );
    }

    let xwallet_account = xwallet_account.map(|account| XWalletAccountInfo {
        sui_object_id: account.sui_object_id,
        x_user_id: account.x_user_id,
        x_handle: account.x_handle,
        owner_address: account.owner_address,
        avatar_url: user_info.profile_image_url.clone(),
    });

    Ok(Json(AuthResponse {
        user: user_info,
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        xwallet_account,
    }))
}

// ====== Refresh Token ======

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: Option<String>,
}

/// Refresh access token using refresh token
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    tracing::info!("Token refresh request received");

    let oauth2_client = TwitterOAuth2Client::new(&state.config);

    let token_response = oauth2_client
        .refresh_token(&request.refresh_token)
        .await
        .map_err(|err| {
            tracing::error!("Token refresh failed: {:?}", err);
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse {
                    error: format!("Token refresh failed: {}", err),
                }),
            )
        })?;

    tracing::info!("Access token refreshed successfully");

    Ok(Json(RefreshTokenResponse {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
    }))
}

// ====== Create Account ======

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub xid: String,
}

#[derive(Debug, Serialize)]
pub struct CreateAccountResponse {
    pub success: bool,
    pub tx_digest: Option<String>,
    pub error: Option<String>,
}

/// Create account endpoint - creates XWallet account for a Twitter user
pub async fn create_account(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, StatusCode> {
    tracing::info!("Create account request for XID: {}", request.xid);

    // Check if account already exists
    match XWalletAccount::find_by_x_user_id(&state.db, &request.xid).await {
        Ok(Some(_)) => {
            tracing::info!("Account already exists for XID: {}", request.xid);
            return Ok(Json(CreateAccountResponse {
                success: false,
                tx_digest: None,
                error: Some("Account already exists".to_string()),
            }));
        }
        Ok(None) => {}
        Err(err) => {
            tracing::error!("Failed to check existing account: {:?}", err);
            return Ok(Json(CreateAccountResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Database error: {}", err)),
            }));
        }
    }

    // Get signed payload from enclave
    let enclave_client = EnclaveClient::new(&state.config.enclave_url);

    let signed_payload = match enclave_client.sign_init_account(&request.xid).await {
        Ok(payload) => payload,
        Err(err) => {
            tracing::error!("Enclave signing failed: {:?}", err);
            return Ok(Json(CreateAccountResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Enclave error: {}", err)),
            }));
        }
    };

    let xid = String::from_utf8_lossy(&signed_payload.response.data.xid).to_string();
    let handle = String::from_utf8_lossy(&signed_payload.response.data.handle).to_string();

    tracing::info!(
        "Enclave signed init account for XID: {}, handle: @{}",
        xid,
        handle
    );

    // Build and submit transaction
    let tx_builder = match SuiTransactionBuilder::new(state.config.clone()).await {
        Ok(builder) => builder,
        Err(err) => {
            tracing::error!("Failed to create transaction builder: {:?}", err);
            return Ok(Json(CreateAccountResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Failed to initialize: {}", err)),
            }));
        }
    };

    match tx_builder
        .init_account(
            &xid,
            &handle,
            signed_payload.response.timestamp_ms,
            &signed_payload.signature,
        )
        .await
    {
        Ok(digest) => {
            tracing::info!("Create account transaction successful: {}", digest);
            Ok(Json(CreateAccountResponse {
                success: true,
                tx_digest: Some(digest),
                error: None,
            }))
        }
        Err(err) => {
            tracing::error!("Create account transaction failed: {:?}", err);
            Ok(Json(CreateAccountResponse {
                success: false,
                tx_digest: None,
                error: Some(format!("Transaction failed: {}", err)),
            }))
        }
    }
}
