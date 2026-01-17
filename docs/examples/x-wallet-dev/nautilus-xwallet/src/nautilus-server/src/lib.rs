// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use fastcrypto::ed25519::Ed25519KeyPair;
use serde_json::json;
use std::fmt;

mod apps {
    #[cfg(feature = "twitter-example")]
    #[path = "twitter-example/mod.rs"]
    pub mod twitter_example;

    #[cfg(feature = "weather-example")]
    #[path = "weather-example/mod.rs"]
    pub mod weather_example;

    #[cfg(feature = "seal-example")]
    #[path = "seal-example/mod.rs"]
    pub mod seal_example;

    #[cfg(feature = "xwallet")]
    #[path = "xwallet/mod.rs"]
    pub mod xwallet;
}

pub mod app {
    #[cfg(feature = "twitter-example")]
    pub use crate::apps::twitter_example::*;

    #[cfg(feature = "weather-example")]
    pub use crate::apps::weather_example::*;

    #[cfg(feature = "seal-example")]
    pub use crate::apps::seal_example::*;

    #[cfg(feature = "xwallet")]
    pub use crate::apps::xwallet::*;
}

pub mod common;

/// App state, at minimum needs to maintain the ephemeral keypair.
pub struct AppState {
    /// Ephemeral keypair on boot
    pub eph_kp: Ed25519KeyPair,
    /// API key (Twitter Bearer Token)
    pub api_key: String,
    /// XWallet Registry ID for NFT name lookup
    pub registry_id: String,
    /// Sui RPC URL for blockchain queries
    pub sui_rpc_url: String,
    /// USDC coin type address
    pub usdc_type: String,
    /// WAL coin type address
    pub wal_type: String,
}

/// Implement IntoResponse for EnclaveError.
impl IntoResponse for EnclaveError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            EnclaveError::GenericError(e) => (StatusCode::BAD_REQUEST, e),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

/// Enclave errors enum.
#[derive(Debug)]
pub enum EnclaveError {
    GenericError(String),
}

impl fmt::Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnclaveError::GenericError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for EnclaveError {}
