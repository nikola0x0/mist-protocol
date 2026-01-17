//! Slack notification client using Incoming Webhooks
//!
//! Simple client to send notifications to a Slack channel when tweets are processed.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn};

/// Slack message payload
#[derive(Debug, Serialize)]
struct SlackMessage {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocks: Option<Vec<SlackBlock>>,
}

/// Slack block for rich formatting
#[derive(Debug, Serialize)]
struct SlackBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<SlackText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<SlackText>>,
}

#[derive(Debug, Serialize)]
struct SlackText {
    #[serde(rename = "type")]
    text_type: String,
    text: String,
}

/// Slack notification client
pub struct SlackClient {
    http_client: Client,
    webhook_url: String,
}

impl SlackClient {
    /// Create a new Slack client with the webhook URL
    pub fn new(webhook_url: String) -> Self {
        Self {
            http_client: Client::new(),
            webhook_url,
        }
    }

    /// Send a simple text notification
    #[allow(dead_code)]
    pub async fn send_notification(&self, message: &str) -> Result<()> {
        let payload = SlackMessage {
            text: message.to_string(),
            blocks: None,
        };

        self.send_payload(&payload).await
    }

    /// Send a transfer success notification with rich formatting
    pub async fn notify_transfer_success(
        &self,
        from_handle: &str,
        to_handle: &str,
        amount: &str,
        coin_type: &str,
        tx_digest: &str,
        tweet_id: &str,
    ) -> Result<()> {
        let blocks = vec![
            SlackBlock {
                block_type: "header".to_string(),
                text: Some(SlackText {
                    text_type: "plain_text".to_string(),
                    text: "Transfer Successful".to_string(),
                }),
                fields: None,
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: None,
                fields: Some(vec![
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*From:*\n@{}", from_handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*To:*\n@{}", to_handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Amount:*\n{} {}", amount, coin_type),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Tweet:*\n<https://x.com/i/web/status/{}|View>", tweet_id),
                    },
                ]),
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: Some(SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!(
                        "*Transaction:*\n<https://suiscan.xyz/testnet/tx/{}|{}>",
                        tx_digest,
                        tx_digest.get(..16).unwrap_or(tx_digest)
                    ),
                }),
                fields: None,
            },
        ];

        let payload = SlackMessage {
            text: format!(
                "Transfer: {} {} from @{} to @{}",
                amount, coin_type, from_handle, to_handle
            ),
            blocks: Some(blocks),
        };

        self.send_payload(&payload).await
    }

    /// Send an account creation notification
    pub async fn notify_account_created(
        &self,
        handle: &str,
        x_user_id: &str,
        tx_digest: &str,
    ) -> Result<()> {
        let blocks = vec![
            SlackBlock {
                block_type: "header".to_string(),
                text: Some(SlackText {
                    text_type: "plain_text".to_string(),
                    text: "New Account Created".to_string(),
                }),
                fields: None,
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: None,
                fields: Some(vec![
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Handle:*\n@{}", handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*X User ID:*\n{}", x_user_id),
                    },
                ]),
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: Some(SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!(
                        "*Transaction:*\n<https://suiscan.xyz/testnet/tx/{}|View on Suiscan>",
                        tx_digest
                    ),
                }),
                fields: None,
            },
        ];

        let payload = SlackMessage {
            text: format!("New account created for @{}", handle),
            blocks: Some(blocks),
        };

        self.send_payload(&payload).await
    }

    /// Send a wallet link notification
    pub async fn notify_wallet_linked(
        &self,
        handle: &str,
        wallet_address: &str,
        _tx_digest: &str,
    ) -> Result<()> {
        let short_address = if wallet_address.len() > 16 {
            format!(
                "{}...{}",
                wallet_address.get(..8).unwrap_or(wallet_address),
                wallet_address.get(wallet_address.len().saturating_sub(6)..).unwrap_or("")
            )
        } else {
            wallet_address.to_string()
        };

        let payload = SlackMessage {
            text: format!("Wallet linked for @{}: {}", handle, short_address),
            blocks: None,
        };

        self.send_payload(&payload).await
    }

    /// Send an error notification
    pub async fn notify_error(
        &self,
        command: &str,
        handle: &str,
        error: &str,
        tweet_id: Option<&str>,
    ) -> Result<()> {
        let tweet_link = tweet_id
            .map(|id| format!(" (<https://x.com/i/web/status/{}|tweet>)", id))
            .unwrap_or_default();

        let payload = SlackMessage {
            text: format!(
                "Command failed: `{}` from @{}{}\nError: {}",
                command, handle, tweet_link, error
            ),
            blocks: None,
        };

        self.send_payload(&payload).await
    }

    /// Send an NFT transfer success notification
    pub async fn notify_nft_transfer_success(
        &self,
        from_handle: &str,
        to_handle: &str,
        nft_id: &str,
        tx_digest: &str,
        tweet_id: &str,
    ) -> Result<()> {
        // Safely truncate NFT ID for display
        let short_nft_id = if nft_id.len() > 20 {
            format!(
                "{}...{}",
                nft_id.get(..10).unwrap_or(nft_id),
                nft_id.get(nft_id.len().saturating_sub(6)..).unwrap_or("")
            )
        } else {
            nft_id.to_string()
        };

        // Safely truncate tx digest for display
        let short_digest = tx_digest.get(..16).unwrap_or(tx_digest);

        let blocks = vec![
            SlackBlock {
                block_type: "header".to_string(),
                text: Some(SlackText {
                    text_type: "plain_text".to_string(),
                    text: "NFT Transfer Successful".to_string(),
                }),
                fields: None,
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: None,
                fields: Some(vec![
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*From:*\n@{}", from_handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*To:*\n@{}", to_handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*NFT:*\n<https://suiscan.xyz/testnet/object/{}|{}>", nft_id, short_nft_id),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Tweet:*\n<https://x.com/i/web/status/{}|View>", tweet_id),
                    },
                ]),
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: Some(SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!(
                        "*Transaction:*\n<https://suiscan.xyz/testnet/tx/{}|{}>",
                        tx_digest, short_digest
                    ),
                }),
                fields: None,
            },
        ];

        let payload = SlackMessage {
            text: format!("NFT Transfer: {} from @{} to @{}", short_nft_id, from_handle, to_handle),
            blocks: Some(blocks),
        };

        self.send_payload(&payload).await
    }

    /// Send a handle update notification
    pub async fn notify_handle_updated(
        &self,
        old_handle: &str,
        new_handle: &str,
        x_user_id: &str,
        tx_digest: &str,
    ) -> Result<()> {
        let blocks = vec![
            SlackBlock {
                block_type: "header".to_string(),
                text: Some(SlackText {
                    text_type: "plain_text".to_string(),
                    text: "Handle Updated".to_string(),
                }),
                fields: None,
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: None,
                fields: Some(vec![
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Old Handle:*\n@{}", old_handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*New Handle:*\n@{}", new_handle),
                    },
                    SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*X User ID:*\n{}", x_user_id),
                    },
                ]),
            },
            SlackBlock {
                block_type: "section".to_string(),
                text: Some(SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: format!(
                        "*Transaction:*\n<https://suiscan.xyz/testnet/tx/{}|View on Suiscan>",
                        tx_digest
                    ),
                }),
                fields: None,
            },
        ];

        let payload = SlackMessage {
            text: format!("Handle updated: @{} -> @{}", old_handle, new_handle),
            blocks: Some(blocks),
        };

        self.send_payload(&payload).await
    }

    /// Send the payload to Slack webhook
    async fn send_payload(&self, payload: &SlackMessage) -> Result<()> {
        let response = self
            .http_client
            .post(&self.webhook_url)
            .json(payload)
            .send()
            .await
            .context("Failed to send Slack notification")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                status = %status,
                body = %body,
                "Slack webhook returned non-success status"
            );
            return Err(anyhow::anyhow!(
                "Slack webhook failed with status {}: {}",
                status,
                body
            ));
        }

        info!("Slack notification sent successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== SlackMessage tests ======

    #[test]
    fn test_slack_message_serialization() {
        let message = SlackMessage {
            text: "Test message".to_string(),
            blocks: None,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Test message"));
    }

    #[test]
    fn test_slack_message_without_blocks_no_blocks_field() {
        let message = SlackMessage {
            text: "Simple text".to_string(),
            blocks: None,
        };
        let json = serde_json::to_string(&message).unwrap();
        // blocks field should not be present when None (skip_serializing_if)
        assert!(!json.contains("blocks"));
    }

    #[test]
    fn test_slack_message_with_blocks() {
        let message = SlackMessage {
            text: "Fallback text".to_string(),
            blocks: Some(vec![SlackBlock {
                block_type: "section".to_string(),
                text: Some(SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: "Block content".to_string(),
                }),
                fields: None,
            }]),
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"blocks\":["));
        assert!(json.contains("\"type\":\"section\""));
        assert!(json.contains("Block content"));
    }

    // ====== SlackBlock tests ======

    #[test]
    fn test_slack_block_header_serialization() {
        let block = SlackBlock {
            block_type: "header".to_string(),
            text: Some(SlackText {
                text_type: "plain_text".to_string(),
                text: "Header Title".to_string(),
            }),
            fields: None,
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"header\""));
        assert!(json.contains("\"type\":\"plain_text\""));
        assert!(json.contains("Header Title"));
    }

    #[test]
    fn test_slack_block_section_with_fields() {
        let block = SlackBlock {
            block_type: "section".to_string(),
            text: None,
            fields: Some(vec![
                SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: "*Field 1:*\nValue 1".to_string(),
                },
                SlackText {
                    text_type: "mrkdwn".to_string(),
                    text: "*Field 2:*\nValue 2".to_string(),
                },
            ]),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"fields\":["));
        assert!(json.contains("Field 1"));
        assert!(json.contains("Field 2"));
    }

    // ====== SlackText tests ======

    #[test]
    fn test_slack_text_mrkdwn() {
        let text = SlackText {
            text_type: "mrkdwn".to_string(),
            text: "*bold* _italic_".to_string(),
        };
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"type\":\"mrkdwn\""));
        assert!(json.contains("*bold* _italic_"));
    }

    #[test]
    fn test_slack_text_plain() {
        let text = SlackText {
            text_type: "plain_text".to_string(),
            text: "Plain text here".to_string(),
        };
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"type\":\"plain_text\""));
    }

    // ====== SlackClient tests ======

    #[test]
    fn test_slack_client_new() {
        let client = SlackClient::new("https://hooks.slack.com/test".to_string());
        assert_eq!(client.webhook_url, "https://hooks.slack.com/test");
    }
}
