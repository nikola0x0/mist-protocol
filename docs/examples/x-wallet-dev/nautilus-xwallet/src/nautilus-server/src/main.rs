// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use axum::{routing::get, routing::post, Router};
use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};
use nautilus_server::app::{
    process_init_account, process_secure_link_wallet, process_tweet, process_update_handle,
};
use nautilus_server::common::{get_attestation, health_check};
use nautilus_server::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing/logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let eph_kp = Ed25519KeyPair::generate(&mut rand::thread_rng());

    // This API_KEY value can be stored with secret-manager. To do that, follow the prompt `sh configure_enclave.sh`
    // Answer `y` to `Do you want to use a secret?` and finish. Otherwise, uncomment this code to use a hardcoded value.
    // let api_key = "045a27812dbe456392913223221306".to_string();
    #[cfg(not(feature = "seal-example"))]
    let api_key = std::env::var("API_KEY").expect("API_KEY must be set");

    // NOTE: if built with `seal-example` flag the `process_data` does not use this api_key from AppState, instead
    // it uses SEAL_API_KEY initialized with two phase bootstrap. Modify this as needed for your application.
    #[cfg(feature = "seal-example")]
    let api_key = String::new();

    // XWallet configuration (loaded from environment variables)
    let sui_rpc_url = std::env::var("SUI_RPC_URL")
        .expect("SUI_RPC_URL must be set (e.g., https://fullnode.testnet.sui.io:443)");
    let usdc_type = std::env::var("USDC_TYPE")
        .expect("USDC_TYPE must be set (e.g., 0x...::usdc::USDC)");
    let wal_type = std::env::var("WAL_TYPE")
        .expect("WAL_TYPE must be set (e.g., 0x...::wal::WAL)");
    let registry_id = std::env::var("XWALLET_REGISTRY_ID").unwrap_or_default();

    info!("XWallet Config:");
    info!("  Sui RPC URL: {}", sui_rpc_url);
    info!("  Registry ID: {}", if registry_id.is_empty() { "(not set)" } else { &registry_id });

    let state = Arc::new(AppState {
        eph_kp,
        api_key,
        registry_id,
        sui_rpc_url,
        usdc_type,
        wal_type,
    });

    // Spawn host-only init server if seal-example feature is enabled
    #[cfg(feature = "seal-example")]
    {
        nautilus_server::app::spawn_host_init_server(state.clone()).await?;
    }

    // Define your own restricted CORS policy here if needed.
    let cors = CorsLayer::new().allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/", get(ping))
        .route("/get_attestation", get(get_attestation))
        // Unified tweet processing endpoint (handles all tweet-based commands)
        .route("/process_tweet", post(process_tweet))
        // Non-tweet endpoints (still needed)
        .route("/process_init_account", post(process_init_account)) // Auto-create recipient by XID
        .route("/process_update_handle", post(process_update_handle)) // Update handle by XID (dApp)
        .route("/process_secure_link_wallet", post(process_secure_link_wallet)) // dApp wallet linking
        .route("/health_check", get(health_check))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}

async fn ping() -> &'static str {
    "Pong!"
}
