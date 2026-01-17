use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{info, warn};

use super::EventHandler;
use crate::indexer::types::{TransferCompletedEvent, SuiEvent};

pub struct TransferCompletedHandler;

#[async_trait]
impl EventHandler for TransferCompletedHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()> {
        // Parse event data
        let parsed_json = event
            .parsed_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing parsed_json in event"))?;

        let event_data: TransferCompletedEvent =
            serde_json::from_value(parsed_json).context("Failed to parse TransferCompleted event")?;

        let amount = event_data
            .amount
            .parse::<i64>()
            .context("Failed to parse amount")?;

        let timestamp = event_data
            .timestamp
            .parse::<i64>()
            .unwrap_or_else(|_| {
                warn!("Failed to parse timestamp for tx {}", event.id.tx_digest);
                chrono::Utc::now().timestamp_millis()
            });

        info!(
            "Handling TransferCompleted: {} -> {}, amount={} {}, tweet_id={}",
            event_data.from_xid,
            event_data.to_xid,
            amount,
            event_data.coin_type,
            event_data.tweet_id
        );

        // Use database transaction for atomicity
        let mut tx = pool.begin().await.context("Failed to start transaction")?;

        // Store transfer in database
        sqlx::query(
            r#"
            INSERT INTO coin_transfers (
                tx_digest,
                transfer_type,
                from_id,
                to_id,
                coin_type,
                amount,
                tweet_id,
                timestamp
            )
            VALUES ($1, 'transfer', $2, $3, $4, $5, $6, $7)
            ON CONFLICT (tx_digest) DO NOTHING
            "#,
        )
        .bind(&event.id.tx_digest)
        .bind(&event_data.from_xid)
        .bind(&event_data.to_xid)
        .bind(&event_data.coin_type)
        .bind(amount)
        .bind(if event_data.tweet_id.is_empty() { None } else { Some(&event_data.tweet_id) })
        .bind(timestamp)
        .execute(&mut *tx)
        .await
        .context("Failed to insert coin transfer")?;

        // Update sender balance (subtract)
        sqlx::query(
            r#"
            INSERT INTO account_balances (x_user_id, coin_type, balance)
            VALUES ($1, $2, 0)
            ON CONFLICT (x_user_id, coin_type)
            DO UPDATE SET
                balance = account_balances.balance - $3,
                updated_at = NOW()
            "#,
        )
        .bind(&event_data.from_xid)
        .bind(&event_data.coin_type)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .context("Failed to update sender balance")?;

        // Update receiver balance (add)
        sqlx::query(
            r#"
            INSERT INTO account_balances (x_user_id, coin_type, balance)
            VALUES ($1, $2, $3)
            ON CONFLICT (x_user_id, coin_type)
            DO UPDATE SET
                balance = account_balances.balance + EXCLUDED.balance,
                updated_at = NOW()
            "#,
        )
        .bind(&event_data.to_xid)
        .bind(&event_data.coin_type)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .context("Failed to update receiver balance")?;

        tx.commit().await.context("Failed to commit transaction")?;

        info!(
            "Updated balances: {} -= {}, {} += {}",
            event_data.from_xid, amount, event_data.to_xid, amount
        );

        Ok(())
    }
}
