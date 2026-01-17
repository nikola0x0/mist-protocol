use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{info, warn};

use super::EventHandler;
use crate::indexer::types::{NftWithdrawnEvent, SuiEvent};

pub struct NftWithdrawnHandler;

#[async_trait]
impl EventHandler for NftWithdrawnHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()> {
        // Parse event data
        let parsed_json = event
            .parsed_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing parsed_json in event"))?;

        let event_data: NftWithdrawnEvent =
            serde_json::from_value(parsed_json).context("Failed to parse NftWithdrawn event")?;

        let timestamp = event
            .timestamp_ms
            .as_ref()
            .and_then(|ts| ts.parse::<i64>().ok())
            .unwrap_or_else(|| {
                warn!("Missing or invalid timestamp for tx {}", event.id.tx_digest);
                chrono::Utc::now().timestamp_millis()
            });

        info!(
            "Handling NftWithdrawn: xid={}, nft_id={}",
            event_data.xid, event_data.nft_id
        );

        // Use database transaction for atomicity
        let mut tx = pool.begin().await.context("Failed to start transaction")?;

        // Get NFT info before deleting
        let nft_info: Option<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT nft_type, name FROM account_nfts WHERE nft_object_id = $1
            "#,
        )
        .bind(&event_data.nft_id)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to fetch NFT info")?;

        let (nft_type, nft_name) = nft_info.unwrap_or(("unknown".to_string(), None));

        // Remove NFT from account_nfts table
        sqlx::query(
            r#"
            DELETE FROM account_nfts
            WHERE nft_object_id = $1
            "#,
        )
        .bind(&event_data.nft_id)
        .execute(&mut *tx)
        .await
        .context("Failed to delete NFT")?;

        // Record NFT withdraw activity (from_id = xid, to_id = NULL)
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
            VALUES ($1, 'withdraw', $2, NULL, $3, $4, $5, $6)
            ON CONFLICT (tx_digest) DO NOTHING
            "#,
        )
        .bind(&event.id.tx_digest)
        .bind(&event_data.xid)  // from_id = sender xid
        .bind(&event_data.nft_id)
        .bind(&nft_type)
        .bind(&nft_name)
        .bind(timestamp)
        .execute(&mut *tx)
        .await
        .context("Failed to insert NFT withdraw")?;

        tx.commit().await.context("Failed to commit transaction")?;

        info!(
            "NFT {} withdrawn from account {}",
            event_data.nft_id, event_data.xid
        );

        Ok(())
    }
}
