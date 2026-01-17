use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::info;

use super::EventHandler;
use crate::db::models::XWalletAccount;
use crate::indexer::types::{HandleUpdatedEvent, SuiEvent};

pub struct HandleUpdatedHandler;

#[async_trait]
impl EventHandler for HandleUpdatedHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()> {
        // Parse event data
        let parsed_json = event
            .parsed_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing parsed_json in event"))?;

        let event_data: HandleUpdatedEvent =
            serde_json::from_value(parsed_json).context("Failed to parse HandleUpdated event")?;

        info!(
            "Handling HandleUpdated: xid={}, {} -> {}",
            event_data.xid, event_data.old_handle, event_data.new_handle
        );

        // Update handle in database
        XWalletAccount::update_handle(pool, &event_data.xid, &event_data.new_handle)
            .await
            .context("Failed to update handle")?;

        Ok(())
    }
}
