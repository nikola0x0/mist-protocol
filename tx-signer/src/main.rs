//! Simple HTTP wrapper around `sui keytool sign`
//! This service runs on port 4000 and signs transactions using the Sui CLI

use axum::{Router, routing::post, Json, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct SignRequest {
    /// Backend address (or alias) to sign with
    address: String,
    /// Base64 encoded transaction data
    tx_data_b64: String,
}

#[derive(Serialize)]
struct SignResponse {
    /// Base64 encoded signature from sui keytool
    signature: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("🔐 Transaction Signing Service");
    println!("   Wraps: sui keytool sign");
    println!("   Port: 4000");
    println!();

    // Check if sui CLI is available
    match std::process::Command::new("sui").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("   ✅ Sui CLI found: {}", version.trim());
        }
        Err(_) => {
            eprintln!("   ❌ ERROR: 'sui' command not found!");
            eprintln!("   Please install Sui CLI: https://docs.sui.io/build/install");
            std::process::exit(1);
        }
    }

    println!();
    println!("   Ready to sign transactions! 🚀");
    println!();

    let app = Router::new()
        .route("/sign", post(sign_transaction))
        .route("/health", axum::routing::get(health_check));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000").await?;
    println!("   Listening on http://127.0.0.1:4000");
    println!();

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn sign_transaction(
    Json(req): Json<SignRequest>,
) -> Result<Json<SignResponse>, (StatusCode, Json<ErrorResponse>)> {
    println!("📝 Signing request for address: {}", req.address);

    // Call sui keytool sign
    let output = std::process::Command::new("sui")
        .args(&[
            "keytool",
            "sign",
            "--address", &req.address,
            "--data", &req.tx_data_b64,
        ])
        .output()
        .map_err(|e| {
            eprintln!("   ❌ Failed to execute sui keytool: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to execute sui keytool: {}", e),
                }),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("   ❌ sui keytool sign failed: {}", stderr);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("sui keytool sign failed: {}", stderr),
            }),
        ));
    }

    // Parse the signature from the output
    let output_str = String::from_utf8_lossy(&output.stdout);

    // The output contains a line like: │ suiSignature │ <base64_signature> │
    let signature = output_str
        .lines()
        .find(|line| line.contains("suiSignature"))
        .and_then(|line| {
            // Split by │ and get the signature part (3rd column)
            line.split('│')
                .nth(2)
                .map(|s| s.trim().to_string())
        })
        .ok_or_else(|| {
            eprintln!("   ❌ Failed to parse signature from output");
            eprintln!("   Output was:\n{}", output_str);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to parse signature from sui keytool output".to_string(),
                }),
            )
        })?;

    println!("   ✅ Transaction signed successfully!");
    println!("   📝 Signature (first 40 chars): {}...", &signature[..40.min(signature.len())]);

    Ok(Json(SignResponse { signature }))
}
