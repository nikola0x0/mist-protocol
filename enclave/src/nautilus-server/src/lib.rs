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
    #[cfg(feature = "mist-protocol")]
    #[path = "mist-protocol/mod.rs"]
    pub mod mist_protocol;

    #[cfg(feature = "mist-protocol")]
    #[path = "flowx/mod.rs"]
    pub mod flowx;
}

pub mod app {
    #[cfg(feature = "mist-protocol")]
    pub use crate::apps::mist_protocol::*;
}

// Export FlowX module for swap_executor
#[cfg(feature = "mist-protocol")]
pub mod flowx {
    pub use crate::apps::flowx::*;
}

pub mod common;

/// App state, at minimum needs to maintain the ephemeral keypair.  
pub struct AppState {
    /// Ephemeral keypair on boot
    pub eph_kp: Ed25519KeyPair,
    /// API key when querying api.weatherapi.com
    pub api_key: String,
}

/// Implement IntoResponse for EnclaveError.
impl IntoResponse for EnclaveError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            EnclaveError::GenericError(e) => (StatusCode::BAD_REQUEST, e),
            EnclaveError::InvalidInput(e) => (StatusCode::BAD_REQUEST, e),
            EnclaveError::DecryptionFailed(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
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
    InvalidInput(String),
    DecryptionFailed(String),
}

impl fmt::Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnclaveError::GenericError(e) => write!(f, "{}", e),
            EnclaveError::InvalidInput(e) => write!(f, "Invalid input: {}", e),
            EnclaveError::DecryptionFailed(e) => write!(f, "Decryption failed: {}", e),
        }
    }
}

impl std::error::Error for EnclaveError {}
