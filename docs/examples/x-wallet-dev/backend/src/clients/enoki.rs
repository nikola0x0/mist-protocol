use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const ENOKI_API_BASE_URL: &str = "https://api.enoki.mystenlabs.com/v1";

#[derive(Clone)]
pub struct EnokiClient {
    api_key: String,
    network: String,
    http: Client,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSponsoredTransactionRequest {
    network: String,
    transaction_block_kind_bytes: String, // base64 encoded
    sender: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnokiDataWrapper<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSponsoredTransactionResponse {
    pub bytes: String,  // base64 encoded sponsored transaction bytes
    pub digest: String, // transaction digest
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteSponsoredTransactionRequest {
    digest: String,
    signature: String, // base64 encoded
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteSponsoredTransactionResponse {
    pub digest: String,
}

impl EnokiClient {
    pub fn new(api_key: String, network: String) -> Self {
        Self {
            api_key,
            network,
            http: Client::new(),
        }
    }

    /// Create a sponsored transaction
    ///
    /// Takes transaction kind bytes (base64) and sender address,
    /// returns sponsored transaction bytes and digest
    pub async fn create_sponsored_transaction(
        &self,
        transaction_block_kind_bytes: String,
        sender: String,
    ) -> Result<CreateSponsoredTransactionResponse> {
        let url = format!("{}/transaction-blocks/sponsor", ENOKI_API_BASE_URL);

        let request = CreateSponsoredTransactionRequest {
            network: self.network.clone(),
            transaction_block_kind_bytes,
            sender,
        };

        tracing::info!("Calling Enoki sponsor API: {}", url);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to call Enoki create sponsored transaction API")?;

        let status = resp.status();
        let response_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());

        tracing::info!("Enoki sponsor response status: {}, body: {}", status, &response_text[..response_text.len().min(500)]);

        if !status.is_success() {
            tracing::error!("Enoki API error (status {}): {}", status, response_text);
            return Err(anyhow!(
                "Enoki API error (status {}): {}",
                status,
                response_text
            ));
        }

        // Try to parse the response - Enoki wraps response in {"data": {...}}
        let wrapper = serde_json::from_str::<EnokiDataWrapper<CreateSponsoredTransactionResponse>>(
            &response_text,
        )
        .map_err(|e| {
            anyhow!(
                "Failed to parse Enoki response: {}. Response body: {}",
                e,
                response_text
            )
        })?;
        Ok(wrapper.data)
    }

    /// Execute a sponsored transaction
    ///
    /// Takes the digest from create_sponsored_transaction and the signature,
    /// submits the transaction to the blockchain
    pub async fn execute_sponsored_transaction(
        &self,
        digest: String,
        signature: String,
    ) -> Result<ExecuteSponsoredTransactionResponse> {
        let url = format!(
            "{}/transaction-blocks/sponsor/{}",
            ENOKI_API_BASE_URL, digest
        );

        let request = ExecuteSponsoredTransactionRequest {
            digest: digest.clone(),
            signature,
        };

        tracing::info!("Calling Enoki execute API: {}", url);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to call Enoki execute sponsored transaction API")?;

        let status = resp.status();
        let response_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());

        tracing::info!("Enoki execute response status: {}, body: {}", status, &response_text[..response_text.len().min(500)]);

        if !status.is_success() {
            tracing::error!("Enoki execute API error (status {}): {}", status, response_text);
            return Err(anyhow!(
                "Enoki API error (status {}): {}",
                status,
                response_text
            ));
        }

        // Try to parse the response - Enoki wraps response in {"data": {...}}
        let wrapper =
            serde_json::from_str::<EnokiDataWrapper<ExecuteSponsoredTransactionResponse>>(
                &response_text,
            )
            .map_err(|e| {
                anyhow!(
                    "Failed to parse Enoki execute response: {}. Response body: {}",
                    e,
                    response_text
                )
            })?;
        Ok(wrapper.data)
    }
}
