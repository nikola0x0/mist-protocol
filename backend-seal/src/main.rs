// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use axum::{routing::get, routing::post, Router};
use fastcrypto::{ed25519::Ed25519KeyPair, traits::KeyPair};
use nautilus_server::app::process_data;
use nautilus_server::common::{get_attestation, health_check};
use nautilus_server::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[cfg(feature = "mist-protocol")]
use nautilus_server::app::seal_test::{decrypt_test, encrypt_test, round_trip_test};

#[tokio::main]
async fn main() -> Result<()> {
    let eph_kp = Ed25519KeyPair::generate(&mut rand::thread_rng());

    // This API_KEY value can be stored with secret-manager. To do that, follow the prompt `sh configure_enclave.sh`
    // Answer `y` to `Do you want to use a secret?` and finish. Otherwise, uncomment this code to use a hardcoded value.
    // let api_key = "045a27812dbe456392913223221306".to_string();

    #[cfg(not(any(feature = "seal-example", feature = "mist-protocol")))]
    let api_key = std::env::var("API_KEY").expect("API_KEY must be set");

    // NOTE: if built with `seal-example` or `mist-protocol` flag, the `process_data` does not use this api_key from AppState
    #[cfg(any(feature = "seal-example", feature = "mist-protocol"))]
    let api_key = String::new();

    let state = Arc::new(AppState { eph_kp, api_key });

    // Spawn host-only init server if seal-example feature is enabled
    #[cfg(feature = "seal-example")]
    {
        nautilus_server::app::spawn_host_init_server(state.clone()).await?;
    }

    // Define your own restricted CORS policy here if needed.
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any); // Allow all origins for development

    let mut app = Router::new()
        .route("/", get(ping))
        .route("/get_attestation", get(get_attestation))
        .route("/process_data", post(process_data))
        .route("/health_check", get(health_check));

    // Add SEAL test endpoints for mist-protocol feature
    #[cfg(feature = "mist-protocol")]
    {
        app = app
            .route("/seal/decrypt", post(decrypt_test))
            .route("/seal/encrypt", post(encrypt_test))
            .route("/seal/round_trip", post(round_trip_test));
        info!("🧪 SEAL test endpoints enabled");
    }

    let app = app.with_state(state).layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    info!("🚀 Backend-seal listening on {}", listener.local_addr().unwrap());
    info!("   Port: 3001 (SEAL Integration)");
    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}

async fn ping() -> &'static str {
    "Pong!"
}
