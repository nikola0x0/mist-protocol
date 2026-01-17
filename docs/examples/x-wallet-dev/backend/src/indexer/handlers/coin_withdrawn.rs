use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{info, warn};

use super::EventHandler;
use crate::indexer::types::{CoinWithdrawnEvent, SuiEvent};

pub struct CoinWithdrawnHandler;

#[async_trait]
impl EventHandler for CoinWithdrawnHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()> {
        // Parse event data
        let parsed_json = event
            .parsed_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing parsed_json in event"))?;

        let event_data: CoinWithdrawnEvent =
            serde_json::from_value(parsed_json).context("Failed to parse CoinWithdrawn event")?;

        let amount = event_data
            .amount
            .parse::<i64>()
            .context("Failed to parse amount")?;

        let timestamp = event
            .timestamp_ms
            .as_ref()
            .and_then(|ts| ts.parse::<i64>().ok())
            .unwrap_or_else(|| {
                warn!("Missing or invalid timestamp for tx {}", event.id.tx_digest);
                chrono::Utc::now().timestamp_millis()
            });

        info!(
            "Handling CoinWithdrawn: xid={}, amount={} {}",
            event_data.xid, amount, event_data.coin_type
        );

        // Use database transaction for atomicity
        let mut tx = pool.begin().await.context("Failed to start transaction")?;

        // Update balance in account_balances table (subtract)
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
        .bind(&event_data.xid)
        .bind(&event_data.coin_type)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .context("Failed to update balance")?;

        // Track in coin_transfers table (from_id = xid, to_id = NULL for now)
        sqlx::query(
            r#"
            INSERT INTO coin_transfers (
                tx_digest,
                transfer_type,
                from_id,
                to_id,
                coin_type,
                amount,
                timestamp
            )
            VALUES ($1, 'withdraw', $2, NULL, $3, $4, $5)
            ON CONFLICT (tx_digest) DO NOTHING
            "#,
        )
        .bind(&event.id.tx_digest)
        .bind(&event_data.xid)  // from_id = sender xid
        .bind(&event_data.coin_type)
        .bind(amount)
        .bind(timestamp)
        .execute(&mut *tx)
        .await
        .context("Failed to insert withdraw transfer")?;

        tx.commit().await.context("Failed to commit transaction")?;

        Ok(())
    }
}
