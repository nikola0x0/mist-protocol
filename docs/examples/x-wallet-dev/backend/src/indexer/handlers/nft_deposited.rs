use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{info, warn};

use super::EventHandler;
use crate::clients::sui_client::SuiClient;
use crate::indexer::types::{NftDepositedEvent, SuiEvent};

pub struct NftDepositedHandler;

#[async_trait]
impl EventHandler for NftDepositedHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()> {
        // Parse event data
        let parsed_json = event
            .parsed_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing parsed_json in event"))?;

        let event_data: NftDepositedEvent =
            serde_json::from_value(parsed_json).context("Failed to parse NftDeposited event")?;

        let timestamp = event
            .timestamp_ms
            .as_ref()
            .and_then(|ts| ts.parse::<i64>().ok())
            .unwrap_or_else(|| {
                warn!("Missing or invalid timestamp for tx {}", event.id.tx_digest);
                chrono::Utc::now().timestamp_millis()
            });

        info!(
            "Handling NftDeposited: xid={}, nft_id={}",
            event_data.xid, event_data.nft_id
        );

        // Fetch NFT metadata from Sui RPC
        let sui_rpc_url = std::env::var("SUI_RPC_URL")
            .unwrap_or_else(|_| "https://fullnode.testnet.sui.io:443".to_string());
        let sui_client = SuiClient::new(&sui_rpc_url);

        let (nft_type, name, image_url) = match sui_client.get_object(&event_data.nft_id).await {
            Ok(Some(obj_data)) => {
                let obj_type = obj_data.object_type.clone().unwrap_or_else(|| "unknown".to_string());
                let name = obj_data.get_name();
                let image_url = obj_data.get_image_url();
                info!(
                    "Fetched NFT metadata: type={}, name={:?}, image_url={:?}",
                    obj_type, name, image_url
                );
                (obj_type, name, image_url)
            }
            Ok(None) => {
                info!("NFT object not found, using defaults");
                ("unknown".to_string(), None, None)
            }
            Err(e) => {
                info!("Failed to fetch NFT metadata: {}, using defaults", e);
                ("unknown".to_string(), None, None)
            }
        };

        // Use database transaction for atomicity
        let mut tx = pool.begin().await.context("Failed to start transaction")?;

        // Insert NFT into account_nfts table
        sqlx::query(
            r#"
            INSERT INTO account_nfts (x_user_id, nft_object_id, nft_type, name, image_url)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (nft_object_id) DO UPDATE SET
                x_user_id = EXCLUDED.x_user_id,
                nft_type = EXCLUDED.nft_type,
                name = EXCLUDED.name,
                image_url = EXCLUDED.image_url,
                updated_at = NOW()
            "#,
        )
        .bind(&event_data.xid)
        .bind(&event_data.nft_id)
        .bind(&nft_type)
        .bind(&name)
        .bind(&image_url)
        .execute(&mut *tx)
        .await
        .context("Failed to insert NFT")?;

        // Record NFT deposit activity (from_id = sender address, to_id = xid)
        sqlx::query(
            r#"
            INSERT INTO nft_transfers (
                tx_digest,
                transfer_type,
                from_id,
                to_id,
                nft_object_id,
                nft_type,
                nft_name,
                timestamp
            )
            VALUES ($1, 'deposit', $2, $3, $4, $5, $6, $7)
            ON CONFLICT (tx_digest) DO NOTHING
            "#,
        )
        .bind(&event.id.tx_digest)
        .bind(&event.sender)  // from_id = external sender address
        .bind(&event_data.xid)  // to_id = receiver xid
        .bind(&event_data.nft_id)
        .bind(&nft_type)
        .bind(&name)
        .bind(timestamp)
        .execute(&mut *tx)
        .await
        .context("Failed to insert NFT deposit")?;

        tx.commit().await.context("Failed to commit transaction")?;

        info!(
            "NFT {} deposited to account {} (type: {}, name: {:?})",
            event_data.nft_id, event_data.xid, nft_type, name
        );

        Ok(())
    }
}
