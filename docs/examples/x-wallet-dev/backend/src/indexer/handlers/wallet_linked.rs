use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::info;

use super::EventHandler;
use crate::db::models::XWalletAccount;
use crate::indexer::types::{SuiEvent, WalletLinkedEvent};

pub struct WalletLinkedHandler;

#[async_trait]
impl EventHandler for WalletLinkedHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()> {
        // Parse event data
        let parsed_json = event
            .parsed_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing parsed_json in event"))?;

        let event_data: WalletLinkedEvent =
            serde_json::from_value(parsed_json).context("Failed to parse WalletLinked event")?;

        info!(
            "Handling WalletLinked: xid={}, owner={}",
            event_data.xid, event_data.owner_address
        );

        // Get the old wallet address before updating (for relink tracking)
        let old_linked_address = XWalletAccount::find_by_x_user_id(pool, &event_data.xid)
            .await
            .ok()
            .flatten()
            .and_then(|acc| acc.owner_address);

        // Update owner_address in database
        XWalletAccount::link_owner(pool, &event_data.xid, &event_data.owner_address)
            .await
            .context("Failed to link owner")?;

        // Record link wallet activity in link_wallet_history table
        // from_address = old wallet (empty string if first time linking)
        // to_address = new wallet being linked
        sqlx::query(
            r#"
            INSERT INTO link_wallet_history (
                tx_digest,
                x_user_id,
                from_address,
                to_address,
                timestamp
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (tx_digest) DO NOTHING
            "#,
        )
        .bind(&event.id.tx_digest)
        .bind(&event_data.xid)
        .bind(old_linked_address.as_deref().unwrap_or(""))  // empty string if first link
        .bind(&event_data.owner_address)
        .bind(
            event
                .timestamp_ms
                .as_ref()
                .and_then(|ts| ts.parse::<i64>().ok())
                .unwrap_or(0),
        )
        .execute(pool)
        .await
        .context("Failed to insert link wallet history")?;

        info!(
            "Wallet linked: {} -> {} for account {}",
            old_linked_address.as_deref().unwrap_or("(none)"),
            event_data.owner_address,
            event_data.xid
        );

        Ok(())
    }
}
