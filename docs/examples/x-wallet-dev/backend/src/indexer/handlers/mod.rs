pub mod account_created;
pub mod coin_deposited;
pub mod coin_transferred;
pub mod coin_withdrawn;
pub mod handle_updated;
pub mod nft_deposited;
pub mod nft_transferred;
pub mod nft_withdrawn;
pub mod wallet_linked;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::indexer::types::SuiEvent;

/// Trait for event handlers
#[async_trait]
pub trait EventHandler {
    async fn handle(pool: &PgPool, event: &SuiEvent) -> Result<()>;
}
