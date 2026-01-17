use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::{debug, warn};

use super::handlers::EventHandler;
use super::types::{parse_event_type, SuiEvent};

pub struct EventProcessor {
    pool: PgPool,
}

impl EventProcessor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process a batch of events
    pub async fn process_events(&self, events: &[SuiEvent]) -> Result<usize> {
        let mut processed = 0;

        for event in events {
            match self.process_single_event(event).await {
                Ok(_) => {
                    processed += 1;
                }
                Err(e) => {
                    warn!("Failed to process event {}: {}", event.id.tx_digest, e);
                    // Continue processing other events
                }
            }
        }

        Ok(processed)
    }

    /// Process a single event
    async fn process_single_event(&self, event: &SuiEvent) -> Result<()> {
        let event_type =
            parse_event_type(&event.event_type).context("Failed to parse event type")?;

        debug!("Processing event: {} ({})", event_type, event.id.tx_digest);

        // Route to appropriate handler based on event type
        match event_type {
            "AccountCreated" => {
                self.handle_account_created(event).await?;
            }
            "WalletLinked" => {
                self.handle_wallet_linked(event).await?;
            }
            "TransferCompleted" => {
                self.handle_transfer_completed(event).await?;
            }
            "CoinDeposited" => {
                self.handle_coin_deposited(event).await?;
            }
            "CoinWithdrawn" => {
                self.handle_coin_withdrawn(event).await?;
            }
            "HandleUpdated" => {
                self.handle_handle_updated(event).await?;
            }
            // NFT events
            "NftDeposited" => {
                self.handle_nft_deposited(event).await?;
            }
            "NftWithdrawn" => {
                self.handle_nft_withdrawn(event).await?;
            }
            "NftTransferCompleted" => {
                self.handle_nft_transferred(event).await?;
            }
            _ => {
                warn!("Unknown event type: {}", event_type);
            }
        }

        Ok(())
    }

    async fn handle_account_created(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::account_created::AccountCreatedHandler;
        AccountCreatedHandler::handle(&self.pool, event).await
    }

    async fn handle_wallet_linked(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::wallet_linked::WalletLinkedHandler;
        WalletLinkedHandler::handle(&self.pool, event).await
    }

    async fn handle_transfer_completed(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::coin_transferred::TransferCompletedHandler;
        TransferCompletedHandler::handle(&self.pool, event).await
    }

    async fn handle_coin_deposited(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::coin_deposited::CoinDepositedHandler;
        CoinDepositedHandler::handle(&self.pool, event).await
    }

    async fn handle_coin_withdrawn(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::coin_withdrawn::CoinWithdrawnHandler;
        CoinWithdrawnHandler::handle(&self.pool, event).await
    }

    async fn handle_handle_updated(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::handle_updated::HandleUpdatedHandler;
        HandleUpdatedHandler::handle(&self.pool, event).await
    }

    async fn handle_nft_deposited(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::nft_deposited::NftDepositedHandler;
        NftDepositedHandler::handle(&self.pool, event).await
    }

    async fn handle_nft_withdrawn(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::nft_withdrawn::NftWithdrawnHandler;
        NftWithdrawnHandler::handle(&self.pool, event).await
    }

    async fn handle_nft_transferred(&self, event: &SuiEvent) -> Result<()> {
        use super::handlers::nft_transferred::NftTransferredHandler;
        NftTransferredHandler::handle(&self.pool, event).await
    }
}
