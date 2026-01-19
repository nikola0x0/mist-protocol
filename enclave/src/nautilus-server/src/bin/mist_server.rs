// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use axum::{routing::get, Router};
use fastcrypto::ed25519::Ed25519KeyPair;
use nautilus_server::common::{get_attestation, health_check};
use nautilus_server::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file (contains BACKEND_PRIVATE_KEY)
    dotenv::dotenv().ok();

    // Initialize tracing subscriber to see logs
    tracing_subscriber::fmt::init();

    println!("✅ Backend starting...");

    // Load backend keypair from environment (persistent, not ephemeral!)
    let backend_kp = load_backend_keypair()?;

    // Log the backend wallet address (only when mist-protocol feature is enabled)
    #[cfg(feature = "mist-protocol")]
    {
        use fastcrypto::traits::ToFromBytes;
        let priv_key_bytes = backend_kp.as_bytes();
        let key_bytes: [u8; 32] = priv_key_bytes[..32].try_into().unwrap();
        let sui_private_key = sui_crypto::ed25519::Ed25519PrivateKey::new(key_bytes);
        let address = sui_private_key.public_key().to_address();

        println!("🔑 Backend Wallet: {}", address);
        println!("🔑 This address is hardcoded in contract for authorization\n");
    }

    // For mist-protocol, we don't need API_KEY
    let api_key = String::new();
    let state = Arc::new(AppState { eph_kp: backend_kp, api_key });

    // Define your own restricted CORS policy here if needed.
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any); // Allow all origins for development

    let app = Router::new()
        .route("/", get(ping))
        .route("/get_attestation", get(get_attestation))
        .route("/health_check", get(health_check))
        .with_state(state.clone())
        .layer(cors);

    // Spawn intent processor background task if mist-protocol feature is enabled
    #[cfg(feature = "mist-protocol")]
    {
        use nautilus_server::app::intent_processor;
        let processor_state = state.clone();
        tokio::spawn(async move {
            intent_processor::start_intent_processor(processor_state).await;
        });
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    println!("🚀 Backend listening on port 3001\n");
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}

async fn ping() -> &'static str {
    "Pong!"
}

/// Load backend keypair from environment variable
///
/// Expects BACKEND_PRIVATE_KEY in Bech32 format (suiprivkey1...)
fn load_backend_keypair() -> Result<Ed25519KeyPair> {
    let private_key_str = std::env::var("BACKEND_PRIVATE_KEY")
        .map_err(|_| anyhow::anyhow!(
            "BACKEND_PRIVATE_KEY not found in environment.\n\
             Generate a wallet with: sui client new-address ed25519\n\
             Then set BACKEND_PRIVATE_KEY=<private_key>"
        ))?;

    // Decode Bech32 private key
    use bech32::FromBase32;
    let (hrp, data, _variant) = bech32::decode(&private_key_str)
        .map_err(|e| anyhow::anyhow!("Invalid Bech32 private key: {}", e))?;

    if hrp != "suiprivkey" {
        return Err(anyhow::anyhow!("Invalid HRP: expected 'suiprivkey', got '{}'", hrp));
    }

    let decoded_bytes = Vec::<u8>::from_base32(&data)
        .map_err(|e| anyhow::anyhow!("Failed to decode base32: {}", e))?;

    // First byte is the scheme (0x00 for ed25519), rest is the 32-byte private key
    if decoded_bytes.len() != 33 {
        return Err(anyhow::anyhow!("Invalid key length: expected 33 bytes, got {}", decoded_bytes.len()));
    }

    if decoded_bytes[0] != 0x00 {
        return Err(anyhow::anyhow!("Invalid key scheme: expected ed25519 (0x00), got 0x{:02x}", decoded_bytes[0]));
    }

    let key_bytes: [u8; 32] = decoded_bytes[1..33]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to extract 32 bytes"))?;

    // Use ToFromBytes trait
    use fastcrypto::traits::ToFromBytes;
    let keypair = Ed25519KeyPair::from_bytes(&key_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to create keypair: {}", e))?;

    Ok(keypair)
}
