use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use tokio::sync::OnceCell;
use tracing::{error, info, warn};

use crate::{
    clients::{
        enclave::{CommandType, EnclaveClient, ProcessTweetResponse},
        redis_client::RedisClient,
        slack::SlackClient,
        sui_transaction::SuiTransactionBuilder,
        twitter::{TransactionResult, TwitterClient},
    },
    constants::coin,
    db::models::{WebhookEvent, XWalletAccount},
    webhook::handler::AppState,
};

/// Guard for account lock - automatically releases on drop
struct AccountLockGuard<'a> {
    redis: &'a RedisClient,
    x_user_id: String,
    token: String,
}

impl<'a> AccountLockGuard<'a> {
    /// Acquire lock with retry, returns None if lock couldn't be acquired
    async fn acquire(redis: &'a RedisClient, x_user_id: &str) -> Result<Option<Self>> {
        match redis.acquire_account_lock_with_retry(x_user_id).await? {
            Some(token) => Ok(Some(Self {
                redis,
                x_user_id: x_user_id.to_string(),
                token,
            })),
            None => Ok(None),
        }
    }

    /// Release the lock explicitly (also called on drop)
    async fn release(self) -> Result<bool> {
        self.redis.release_account_lock(&self.x_user_id, &self.token).await
    }
}

/// Simple transaction processor worker (SIMPLIFIED ARCHITECTURE):
/// 1. pop queue item from Redis
/// 2. call enclave /process_tweet endpoint (Nautilus parses command)
/// 3. route based on response.command_type
/// 4. submit Sui transaction
/// 5. reply to tweet with success/error message
/// 6. mark webhook event processed
pub struct ProcessorWorker {
    state: Arc<AppState>,
    enclave: EnclaveClient,
    redis: RedisClient,
    twitter: TwitterClient,
    /// Optional Slack client for notifications
    slack: Option<SlackClient>,
    /// Lazily initialized, reusable transaction builder
    tx_builder: OnceCell<SuiTransactionBuilder>,
}

impl ProcessorWorker {
    pub fn new(state: Arc<AppState>) -> Self {
        let enclave = EnclaveClient::new(state.config.enclave_url.clone());
        let redis = state.redis.clone();
        let twitter = TwitterClient::new(&state.config);

        // Initialize Slack client if webhook URL is configured
        let slack = state.config.slack_webhook_url.as_ref().map(|url| {
            info!("Slack notifications enabled");
            SlackClient::new(url.clone())
        });

        Self {
            state,
            enclave,
            redis,
            twitter,
            slack,
            tx_builder: OnceCell::new(),
        }
    }

    /// Get or initialize the transaction builder (lazy initialization, reused across requests)
    async fn get_tx_builder(&self) -> Result<&SuiTransactionBuilder> {
        self.tx_builder
            .get_or_try_init(|| async {
                SuiTransactionBuilder::new(self.state.config.clone()).await
            })
            .await
            .context("Failed to initialize Sui transaction builder")
    }

    pub async fn run(self) {
        info!("Starting transaction processor worker");

        loop {
            match self.process_once().await {
                Ok(ProcessOutcome::Empty) => {
                    // Idle wait to avoid busy loop
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Ok(ProcessOutcome::Processed { event_id, tweet_id }) => {
                    info!(%event_id, %tweet_id, "Processed tweet event");
                }
                Err(err) => {
                    error!("Processor error: {:#}", err);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_once(&self) -> Result<ProcessOutcome> {
        // Pop oldest tweet from sorted queues (ordered by tweet timestamp)
        let result = self
            .redis
            .pop_oldest_from_sorted_queues_blocking(1)
            .await
            .context("failed popping sorted user queue")?;

        let Some((_xid, raw, tweet_timestamp)) = result else {
            return Ok(ProcessOutcome::Empty);
        };

        info!("Processing tweet with timestamp: {} ({})", tweet_timestamp,
            chrono::DateTime::from_timestamp_millis(tweet_timestamp as i64)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "invalid".to_string())
        );

        let item: QueueItem =
            serde_json::from_str(&raw).context("failed to parse queue item JSON")?;

        // Fetch webhook event for context
        let event = WebhookEvent::find_by_event_id(&self.state.db, &item.event_id)
            .await
            .context("failed to fetch webhook event")?;

        let _event = if let Some(event) = event {
            if event.is_done() {
                info!(event_id = %item.event_id, status = ?event.status, "Webhook event already done, skipping");
                return Ok(ProcessOutcome::Processed {
                    event_id: item.event_id,
                    tweet_id: item.tweet_id,
                });
            }
            event
        } else {
            warn!(event_id = %item.event_id, "Webhook event not found, skipping");
            return Ok(ProcessOutcome::Processed {
                event_id: item.event_id,
                tweet_id: item.tweet_id,
            });
        };

        // Set status to processing
        WebhookEvent::set_processing(&self.state.db, &item.event_id)
            .await
            .context("failed to set event to processing")?;

        // Build tweet URL from tweet_id
        let tweet_url = format!("https://x.com/user/status/{}", item.tweet_id);

        info!(tweet_url = %tweet_url, event_id = %item.event_id, "Calling unified /process_tweet endpoint");

        // Call unified /process_tweet endpoint - Nautilus handles all parsing
        let process_result = match self.enclave.process_tweet(&tweet_url).await {
            Ok(result) => result,
            Err(e) => {
                // Get full error chain (includes root cause with MoveAbort code)
                let full_error = format!("{:#}", e);
                error!(event_id = %item.event_id, error = %full_error, "Enclave process_tweet failed");

                // Reply to tweet with error (command passed regex filter but enclave failed)
                if let Err(reply_err) = self.twitter.reply_error(&item.tweet_id, &full_error).await {
                    warn!(error = %reply_err, "Failed to reply to tweet with enclave error");
                }

                // Send Slack notification for error
                if let Some(slack) = &self.slack {
                    if let Err(slack_err) = slack.notify_error(
                        "process_tweet",
                        "unknown",
                        &full_error,
                        Some(&item.tweet_id),
                    ).await {
                        warn!(error = %slack_err, "Failed to send Slack error notification");
                    }
                }

                WebhookEvent::set_failed(&self.state.db, &item.event_id, &full_error)
                    .await
                    .context("failed to set event to failed")?;
                return Ok(ProcessOutcome::Processed {
                    event_id: item.event_id,
                    tweet_id: item.tweet_id,
                });
            }
        };

        info!(
            command_type = ?process_result.command_type,
            tweet_id = %process_result.common.tweet_id,
            author = %process_result.common.author_handle,
            "Received response from process_tweet"
        );

        // Route based on command_type from Nautilus response
        let result = match process_result.command_type {
            CommandType::CreateAccount => {
                self.handle_create_account(&process_result, &item.tweet_id, &item.event_id)
                    .await
            }
            CommandType::Transfer => {
                self.handle_transfer(&process_result, &item.tweet_id, &item.event_id)
                    .await
            }
            CommandType::LinkWallet => {
                self.handle_link_wallet(&process_result, &item.tweet_id, &item.event_id)
                    .await
            }
            CommandType::TransferNft => {
                self.handle_nft_transfer(&process_result, &item.tweet_id, &item.event_id)
                    .await
            }
            CommandType::UpdateHandle => {
                self.handle_update_handle(&process_result, &item.tweet_id, &item.event_id)
                    .await
            }
        };

        // Handle result
        if let Err(e) = result {
            // Get full error chain (includes root cause with MoveAbort code)
            let full_error = format!("{:#}", e);
            error!(event_id = %item.event_id, error = %full_error, "Failed to process event");

            // Reply to tweet with error (command passed regex filter but processing failed)
            // Use full_error so we can extract MoveAbort codes
            if let Err(reply_err) = self.twitter.reply_error(&item.tweet_id, &full_error).await {
                warn!(error = %reply_err, "Failed to reply to tweet with processing error");
            }

            // Send Slack notification for error with command type and author info
            if let Some(slack) = &self.slack {
                let command_name = format!("{:?}", process_result.command_type);
                if let Err(slack_err) = slack.notify_error(
                    &command_name,
                    &process_result.common.author_handle,
                    &full_error,
                    Some(&item.tweet_id),
                ).await {
                    warn!(error = %slack_err, "Failed to send Slack error notification");
                }
            }

            WebhookEvent::set_failed(&self.state.db, &item.event_id, &full_error)
                .await
                .context("failed to set event to failed")?;
        }

        Ok(ProcessOutcome::Processed {
            event_id: item.event_id,
            tweet_id: item.tweet_id,
        })
    }

    // ========================================================================
    // NEW: Handlers for unified /process_tweet response (simplified architecture)
    // ========================================================================

    /// Handle create account command from process_tweet response
    async fn handle_create_account(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
    ) -> Result<()> {
        let data = EnclaveClient::parse_create_account_data(response)
            .context("Failed to parse create account data")?;

        info!(
            xid = %data.xid,
            handle = %data.handle,
            timestamp = response.timestamp_ms,
            "Handling create account command"
        );

        // Status: submitting
        WebhookEvent::set_submitting(&self.state.db, event_id)
            .await
            .context("Failed to set event to submitting")?;

        // Get reusable transaction builder
        let tx_builder = self.get_tx_builder().await?;

        // Submit transaction with enclave signature
        let digest = tx_builder
            .init_account(&data.xid, &data.handle, response.timestamp_ms, &response.signature)
            .await
            .context("Failed to submit init account transaction")?;

        info!(
            tx_digest = %digest,
            "Account initialized successfully for XID: {}", data.xid
        );

        // Status: replying
        WebhookEvent::set_replying(&self.state.db, event_id, &digest)
            .await
            .context("Failed to set event to replying")?;

        // Reply to tweet with success message
        if let Err(e) = self
            .twitter
            .reply_account_created(tweet_id, &data.handle, &digest)
            .await
        {
            warn!(error = %e, "Failed to reply to tweet with account creation success");
        }

        // Send Slack notification
        if let Some(slack) = &self.slack {
            if let Err(e) = slack
                .notify_account_created(&data.handle, &data.xid, &digest)
                .await
            {
                warn!(error = %e, "Failed to send Slack notification for account creation");
            }
        }

        // Status: completed
        WebhookEvent::set_completed(&self.state.db, event_id)
            .await
            .context("Failed to set event to completed")?;

        Ok(())
    }

    /// Handle update handle command from process_tweet response
    async fn handle_update_handle(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
    ) -> Result<()> {
        let data = EnclaveClient::parse_update_handle_data(response)
            .context("Failed to parse update handle data")?;

        info!(
            xid = %data.xid,
            old_handle = %data.old_handle,
            new_handle = %data.new_handle,
            timestamp = response.timestamp_ms,
            "Handling update handle command"
        );

        // Status: submitting
        WebhookEvent::set_submitting(&self.state.db, event_id)
            .await
            .context("Failed to set event to submitting")?;

        // Get reusable transaction builder
        let tx_builder = self.get_tx_builder().await?;

        // Submit transaction with enclave signature
        let digest = tx_builder
            .update_handle(&data.xid, &data.new_handle, response.timestamp_ms, &response.signature)
            .await
            .context("Failed to submit update handle transaction")?;

        info!(
            tx_digest = %digest,
            "Handle updated successfully for XID: {} from @{} to @{}", data.xid, data.old_handle, data.new_handle
        );

        // Status: replying
        WebhookEvent::set_replying(&self.state.db, event_id, &digest)
            .await
            .context("Failed to set event to replying")?;

        // Reply to tweet with success message
        if let Err(e) = self
            .twitter
            .reply_handle_updated(tweet_id, &data.old_handle, &data.new_handle, &digest)
            .await
        {
            warn!(error = %e, "Failed to reply to tweet with handle update success");
        }

        // Send Slack notification
        if let Some(slack) = &self.slack {
            if let Err(e) = slack
                .notify_handle_updated(&data.old_handle, &data.new_handle, &data.xid, &digest)
                .await
            {
                warn!(error = %e, "Failed to send Slack notification for handle update");
            }
        }

        // Status: completed
        WebhookEvent::set_completed(&self.state.db, event_id)
            .await
            .context("Failed to set event to completed")?;

        Ok(())
    }

    /// Handle transfer command from process_tweet response
    /// Uses distributed lock to ensure ordering guarantee per account
    async fn handle_transfer(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
    ) -> Result<()> {
        let data = EnclaveClient::parse_transfer_data(response)
            .context("Failed to parse transfer data")?;

        info!(
            from_xid = %data.from_xid,
            to_xid = %data.to_xid,
            amount = data.amount,
            coin_type = %data.coin_type,
            timestamp = response.timestamp_ms,
            "Handling transfer command"
        );

        // Acquire distributed lock on sender account for ordering guarantee
        let lock = AccountLockGuard::acquire(&self.redis, &data.from_xid)
            .await
            .context("Failed to acquire account lock")?;

        let lock = match lock {
            Some(l) => l,
            None => {
                return Err(anyhow!(
                    "Failed to acquire lock for account {} - another transaction may be in progress",
                    data.from_xid
                ));
            }
        };

        info!(from_xid = %data.from_xid, "Acquired account lock for transfer");

        // Execute transfer with lock held
        let result = self
            .execute_transfer_with_lock(response, tweet_id, event_id, &data)
            .await;

        // Release lock after transaction (regardless of success/failure)
        if let Err(e) = lock.release().await {
            warn!(error = %e, from_xid = %data.from_xid, "Failed to release account lock");
        } else {
            info!(from_xid = %data.from_xid, "Released account lock");
        }

        result
    }

    /// Execute transfer with lock already held
    async fn execute_transfer_with_lock(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
        data: &crate::clients::enclave::TransferData,
    ) -> Result<()> {
        // Check if recipient account exists, create if not
        let recipient_exists =
            XWalletAccount::find_by_x_user_id(&self.state.db, &data.to_xid)
                .await
                .context("Failed to check if recipient account exists")?
                .is_some();

        let recipient_just_created = if !recipient_exists {
            info!(to_xid = %data.to_xid, "Recipient account does not exist, creating account first");
            self.auto_create_recipient_account(&data.to_xid)
                .await
                .context("Failed to auto-create recipient account")?;
            true
        } else {
            false
        };

        // Status: submitting
        WebhookEvent::set_submitting(&self.state.db, event_id)
            .await
            .context("Failed to set event to submitting")?;

        // Get reusable transaction builder
        let tx_builder = self.get_tx_builder().await?;

        // Submit transaction with enclave signature
        let digest = tx_builder
            .submit_transfer(
                &data.from_xid,
                &data.to_xid,
                data.amount,
                &data.coin_type,
                &response.common.tweet_id,
                response.timestamp_ms,
                &response.signature,
                recipient_just_created,
            )
            .await
            .context("Failed to submit transfer transaction")?;

        info!(
            tx_digest = %digest,
            "Transfer transaction submitted successfully"
        );

        // Increment sequence number after successful transaction
        let new_sequence = XWalletAccount::increment_sequence(&self.state.db, &data.from_xid)
            .await
            .context("Failed to increment account sequence")?;

        info!(
            from_xid = %data.from_xid,
            new_sequence = new_sequence,
            "Account sequence incremented"
        );

        // Status: replying
        WebhookEvent::set_replying(&self.state.db, event_id, &digest)
            .await
            .context("Failed to set event to replying")?;

        // Reply to tweet with success message
        let tx_result = TransactionResult {
            tx_digest: digest,
            from_handle: data.from_handle.clone(),
            to_handle: data.to_handle.clone(),
            amount: data.amount,
            coin_type: data.coin_type.clone(),
            original_tweet_id: tweet_id.to_string(),
        };

        if let Err(e) = self.twitter.reply_transfer_success(&tx_result).await {
            warn!(error = %e, "Failed to reply to tweet with transfer success");
        }

        // Send Slack notification
        if let Some(slack) = &self.slack {
            let coin_info = coin::get_coin_info(&data.coin_type);
            let amount_formatted = coin::format_amount(data.amount, coin_info.decimals);

            if let Err(e) = slack
                .notify_transfer_success(
                    &data.from_handle,
                    &data.to_handle,
                    &amount_formatted,
                    coin_info.symbol,
                    &tx_result.tx_digest,
                    tweet_id,
                )
                .await
            {
                warn!(error = %e, "Failed to send Slack notification for transfer");
            }
        }

        // Status: completed
        WebhookEvent::set_completed(&self.state.db, event_id)
            .await
            .context("Failed to set event to completed")?;

        Ok(())
    }

    /// Handle link wallet command from process_tweet response
    async fn handle_link_wallet(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
    ) -> Result<()> {
        let data = EnclaveClient::parse_link_wallet_data(response)
            .context("Failed to parse link wallet data")?;

        info!(
            xid = %data.xid,
            wallet = %data.wallet_address,
            timestamp = response.timestamp_ms,
            "Handling link wallet command"
        );

        // Status: submitting
        WebhookEvent::set_submitting(&self.state.db, event_id)
            .await
            .context("Failed to set event to submitting")?;

        // Get reusable transaction builder
        let tx_builder = self.get_tx_builder().await?;

        // Submit link wallet transaction with enclave signature
        let digest = tx_builder
            .link_wallet(&data.xid, &data.wallet_address, response.timestamp_ms, &response.signature)
            .await
            .context("Failed to submit link wallet transaction")?;

        info!(
            tx_digest = %digest,
            "Wallet linked successfully for XID: {} to address: {}", data.xid, data.wallet_address
        );

        // Status: replying
        WebhookEvent::set_replying(&self.state.db, event_id, &digest)
            .await
            .context("Failed to set event to replying")?;

        // Reply to tweet with wallet linking success
        self.twitter
            .reply_wallet_linked(
                tweet_id,
                &response.common.author_handle,
                &data.wallet_address,
                &digest,
            )
            .await
            .context("Failed to reply with wallet linking success")?;

        // Send Slack notification
        if let Some(slack) = &self.slack {
            if let Err(e) = slack
                .notify_wallet_linked(&response.common.author_handle, &data.wallet_address, &digest)
                .await
            {
                warn!(error = %e, "Failed to send Slack notification for wallet linking");
            }
        }

        // Status: completed
        WebhookEvent::set_completed(&self.state.db, event_id)
            .await
            .context("Failed to set event to completed")?;

        Ok(())
    }

    /// Handle NFT transfer command from process_tweet response
    /// Uses distributed lock to ensure ordering guarantee per account
    async fn handle_nft_transfer(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
    ) -> Result<()> {
        let data = EnclaveClient::parse_transfer_nft_data(response)
            .context("Failed to parse transfer NFT data")?;

        info!(
            from_xid = %data.from_xid,
            to_xid = %data.to_xid,
            nft_id = %data.nft_id,
            timestamp = response.timestamp_ms,
            "Handling NFT transfer command"
        );

        // Acquire distributed lock on sender account for ordering guarantee
        let lock = AccountLockGuard::acquire(&self.redis, &data.from_xid)
            .await
            .context("Failed to acquire account lock")?;

        let lock = match lock {
            Some(l) => l,
            None => {
                return Err(anyhow!(
                    "Failed to acquire lock for account {} - another transaction may be in progress",
                    data.from_xid
                ));
            }
        };

        info!(from_xid = %data.from_xid, "Acquired account lock for NFT transfer");

        // Execute NFT transfer with lock held
        let result = self
            .execute_nft_transfer_with_lock(response, tweet_id, event_id, &data)
            .await;

        // Release lock after transaction (regardless of success/failure)
        if let Err(e) = lock.release().await {
            warn!(error = %e, from_xid = %data.from_xid, "Failed to release account lock");
        } else {
            info!(from_xid = %data.from_xid, "Released account lock");
        }

        result
    }

    /// Execute NFT transfer with lock already held
    async fn execute_nft_transfer_with_lock(
        &self,
        response: &ProcessTweetResponse,
        tweet_id: &str,
        event_id: &str,
        data: &crate::clients::enclave::TransferNftData,
    ) -> Result<()> {
        // Check if recipient account exists, create if not
        let recipient_exists =
            XWalletAccount::find_by_x_user_id(&self.state.db, &data.to_xid)
                .await
                .context("Failed to check if recipient account exists")?
                .is_some();

        let recipient_just_created = if !recipient_exists {
            info!(to_xid = %data.to_xid, "Recipient account does not exist, creating account first");
            self.auto_create_recipient_account(&data.to_xid)
                .await
                .context("Failed to auto-create recipient account")?;
            true
        } else {
            false
        };

        // Status: submitting
        WebhookEvent::set_submitting(&self.state.db, event_id)
            .await
            .context("Failed to set event to submitting")?;

        // Get reusable transaction builder
        let tx_builder = self.get_tx_builder().await?;

        // Submit NFT transfer transaction with enclave signature
        let digest = tx_builder
            .submit_nft_transfer(
                &data.from_xid,
                &data.to_xid,
                &data.nft_id,
                &response.common.tweet_id,
                response.timestamp_ms,
                &response.signature,
                recipient_just_created,
            )
            .await
            .context("Failed to submit NFT transfer transaction")?;

        info!(
            tx_digest = %digest,
            "NFT transfer transaction submitted successfully"
        );

        // Increment sequence number after successful transaction
        let new_sequence = XWalletAccount::increment_sequence(&self.state.db, &data.from_xid)
            .await
            .context("Failed to increment account sequence")?;

        info!(
            from_xid = %data.from_xid,
            new_sequence = new_sequence,
            "Account sequence incremented after NFT transfer"
        );

        // Status: replying
        WebhookEvent::set_replying(&self.state.db, event_id, &digest)
            .await
            .context("Failed to set event to replying")?;

        // Reply to tweet with success message
        if let Err(e) = self
            .twitter
            .reply_nft_transfer_success(
                tweet_id,
                &data.from_handle,
                &data.to_handle,
                &data.nft_id,
                &digest,
            )
            .await
        {
            warn!(error = %e, "Failed to reply to tweet with NFT transfer success");
        }

        // Send Slack notification
        if let Some(slack) = &self.slack {
            if let Err(e) = slack
                .notify_nft_transfer_success(
                    &data.from_handle,
                    &data.to_handle,
                    &data.nft_id,
                    &digest,
                    tweet_id,
                )
                .await
            {
                warn!(error = %e, "Failed to send Slack notification for NFT transfer");
            }
        }

        // Status: completed
        WebhookEvent::set_completed(&self.state.db, event_id)
            .await
            .context("Failed to set event to completed")?;

        Ok(())
    }

    /// Get Twitter handle from database or return XID as fallback
    /// Note: This is kept for potential future use but currently unused
    /// since handles come from ProcessTweetResponse
    #[allow(dead_code)]
    async fn get_x_handle(&self, xid: &str) -> Result<String> {
        let account =
            crate::db::models::XWalletAccount::find_by_x_user_id(&self.state.db, xid)
                .await
                .context("Failed to fetch account")?;

        match account {
            Some(acc) => Ok(acc.x_handle),
            None => Ok(xid.to_string()),
        }
    }

    /// Auto-create account for recipient who doesn't have an XWallet account yet
    async fn auto_create_recipient_account(
        &self,
        to_xid: &str,
    ) -> Result<()> {
        info!(to_xid = %to_xid, "Auto-creating account for recipient via Nautilus enclave");

        // Call Nautilus enclave to sign init account for the recipient
        let signed = self
            .enclave
            .sign_init_account(to_xid)
            .await
            .context("Failed to sign init account for recipient")?;

        let xid = String::from_utf8(signed.response.data.xid.clone())
            .context("Invalid xid encoding from enclave")?;
        let handle = String::from_utf8(signed.response.data.handle.clone())
            .context("Invalid handle encoding from enclave")?;

        info!(
            xid = %xid,
            handle = %handle,
            timestamp = signed.response.timestamp_ms,
            "Submitting auto-created account initialization to Sui with enclave signature"
        );

        // Get reusable transaction builder
        let tx_builder = self.get_tx_builder().await?;

        // Submit init account transaction with enclave signature
        let digest = tx_builder
            .init_account(&xid, &handle, signed.response.timestamp_ms, &signed.signature)
            .await
            .context("Failed to submit auto-created account init transaction")?;

        info!(
            tx_digest = %digest,
            to_xid = %to_xid,
            "Recipient account auto-created successfully"
        );

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct QueueItem {
    tweet_id: String,
    event_id: String,
}

enum ProcessOutcome {
    Empty,
    Processed { event_id: String, tweet_id: String },
}

// NOTE: parse_tweet_command has been REMOVED
// Tweet parsing is now done entirely in Nautilus enclave via /process_tweet endpoint
// This simplifies backend logic and centralizes all tweet parsing in one place

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_status_is_done() {
        use crate::db::models::EventStatus;

        let completed = WebhookEvent {
            id: 1,
            event_id: "test".to_string(),
            tweet_id: None,
            payload: serde_json::json!({}),
            status: EventStatus::Completed,
            tx_digest: None,
            error_message: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert!(completed.is_done());

        let failed = WebhookEvent {
            status: EventStatus::Failed,
            ..completed.clone()
        };
        assert!(failed.is_done());

        let pending = WebhookEvent {
            status: EventStatus::Pending,
            ..completed.clone()
        };
        assert!(!pending.is_done());

        let processing = WebhookEvent {
            status: EventStatus::Processing,
            ..completed
        };
        assert!(!processing.is_done());
    }

    #[test]
    fn test_command_type_deserialization() {
        // Test that CommandType deserializes correctly from JSON
        let json = r#""create_account""#;
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::CreateAccount);

        let json = r#""transfer""#;
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::Transfer);

        let json = r#""link_wallet""#;
        let cmd: CommandType = serde_json::from_str(json).unwrap();
        assert_eq!(cmd, CommandType::LinkWallet);
    }
}
