use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct XWalletAccount {
    pub id: i32,
    pub x_user_id: String,
    pub x_handle: String,
    pub sui_object_id: String,
    pub owner_address: Option<String>,
    pub avatar_url: Option<String>,
    pub last_timestamp: i64,
    /// Monotonically increasing counter for transaction ordering
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AccountNft {
    pub id: i32,
    pub x_user_id: String,
    pub nft_object_id: String,
    pub nft_type: String,
    pub name: Option<String>,
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AccountBalance {
    pub id: i32,
    pub x_user_id: String,
    pub coin_type: String,
    pub balance: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AccountBalance {
    pub async fn find_by_x_user_id(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AccountBalance>(
            r#"
            SELECT id, x_user_id, coin_type, balance, created_at, updated_at
            FROM account_balances
            WHERE x_user_id = $1
            ORDER BY coin_type
            "#
        )
        .bind(x_user_id)
        .fetch_all(pool)
        .await
    }
}


/// Event status enum matching PostgreSQL event_status type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "event_status", rename_all = "lowercase")]
pub enum EventStatus {
    Pending,     // Đã nhận, chờ xử lý
    Processing,  // Đang parse/xử lý
    Submitting,  // Đang submit PTB lên Sui
    Replying,    // Submit xong, đang reply tweet
    Completed,   // Hoàn tất
    Failed,      // Thất bại
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: i32,
    pub event_id: String,
    pub tweet_id: Option<String>,
    pub payload: serde_json::Value,
    pub status: EventStatus,
    pub tx_digest: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct IndexerState {
    pub id: i32,
    pub name: String,
    pub cursor: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl XWalletAccount {
    #[allow(dead_code)]
    pub async fn create(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        x_handle: &str,
        sui_object_id: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            INSERT INTO xwallet_accounts (x_user_id, x_handle, sui_object_id)
            VALUES ($1, $2, $3)
            RETURNING id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            "#
        )
        .bind(x_user_id)
        .bind(x_handle)
        .bind(sui_object_id)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_x_user_id(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            SELECT id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            FROM xwallet_accounts
            WHERE x_user_id = $1
            "#
        )
        .bind(x_user_id)
        .fetch_optional(pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn find_by_x_handle(
        pool: &sqlx::PgPool,
        x_handle: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        // Remove @ prefix if present
        let clean_handle = x_handle.trim_start_matches('@');
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            SELECT id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            FROM xwallet_accounts
            WHERE LOWER(x_handle) = LOWER($1)
            "#
        )
        .bind(clean_handle)
        .fetch_optional(pool)
        .await
    }

    #[allow(dead_code)]
    pub async fn find_by_sui_object_id(
        pool: &sqlx::PgPool,
        sui_object_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            SELECT id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            FROM xwallet_accounts
            WHERE sui_object_id = $1
            "#
        )
        .bind(sui_object_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert_from_indexer(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        x_handle: &str,
        sui_object_id: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            INSERT INTO xwallet_accounts (x_user_id, x_handle, sui_object_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (x_user_id)
            DO UPDATE SET
                x_handle = EXCLUDED.x_handle,
                sui_object_id = EXCLUDED.sui_object_id,
                updated_at = NOW()
            RETURNING id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            "#
        )
        .bind(x_user_id)
        .bind(x_handle)
        .bind(sui_object_id)
        .fetch_one(pool)
        .await
    }

    pub async fn update_handle(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        new_handle: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            UPDATE xwallet_accounts
            SET x_handle = $2, updated_at = NOW()
            WHERE x_user_id = $1
            RETURNING id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            "#
        )
        .bind(x_user_id)
        .bind(new_handle)
        .fetch_optional(pool)
        .await
    }

    pub async fn link_owner(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        owner_address: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            UPDATE xwallet_accounts
            SET owner_address = $2, updated_at = NOW()
            WHERE x_user_id = $1
            RETURNING id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            "#
        )
        .bind(x_user_id)
        .bind(owner_address)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_avatar(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        avatar_url: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE xwallet_accounts
            SET avatar_url = $2, updated_at = NOW()
            WHERE x_user_id = $1
            "#
        )
        .bind(x_user_id)
        .bind(avatar_url)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_owner_address(
        pool: &sqlx::PgPool,
        owner_address: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            SELECT id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            FROM xwallet_accounts
            WHERE owner_address = $1
            "#
        )
        .bind(owner_address)
        .fetch_optional(pool)
        .await
    }

    pub async fn search(pool: &sqlx::PgPool, query: &str) -> Result<Vec<Self>, sqlx::Error> {
        // Remove @ prefix if present
        let clean_query = query.trim_start_matches('@');

        sqlx::query_as::<_, XWalletAccount>(
            r#"
            SELECT id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            FROM xwallet_accounts
            WHERE x_handle ILIKE $1
               OR x_user_id = $2
               OR sui_object_id = $2
               OR owner_address = $2
            ORDER BY
                CASE
                    WHEN x_handle ILIKE $1 THEN 1
                    WHEN x_user_id = $2 THEN 2
                    ELSE 3
                END,
                x_handle
            LIMIT 20
            "#
        )
        .bind(format!("%{}%", clean_query))
        .bind(query)
        .fetch_all(pool)
        .await
    }

    /// Batch lookup handles by XIDs - returns HashMap<xid, handle>
    pub async fn find_handles_by_xids(
        pool: &sqlx::PgPool,
        xids: &[String],
    ) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
        if xids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let accounts = sqlx::query_as::<_, XWalletAccount>(
            r#"
            SELECT id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            FROM xwallet_accounts
            WHERE x_user_id = ANY($1)
            "#,
        )
        .bind(xids)
        .fetch_all(pool)
        .await?;

        Ok(accounts
            .into_iter()
            .map(|acc| (acc.x_user_id, acc.x_handle))
            .collect())
    }

    #[allow(dead_code)]
    pub async fn update_last_timestamp(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        last_timestamp: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, XWalletAccount>(
            r#"
            UPDATE xwallet_accounts
            SET last_timestamp = $2, updated_at = NOW()
            WHERE x_user_id = $1
            RETURNING id, x_user_id, x_handle, sui_object_id, owner_address, avatar_url, last_timestamp, sequence, created_at, updated_at
            "#
        )
        .bind(x_user_id)
        .bind(last_timestamp)
        .fetch_optional(pool)
        .await
    }

    /// Increment sequence number atomically and return new value
    /// This should be called within a distributed lock context
    pub async fn increment_sequence(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            UPDATE xwallet_accounts
            SET sequence = sequence + 1, updated_at = NOW()
            WHERE x_user_id = $1
            RETURNING sequence
            "#
        )
        .bind(x_user_id)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get current sequence number without incrementing
    #[allow(dead_code)]
    pub async fn get_sequence(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT sequence FROM xwallet_accounts WHERE x_user_id = $1
            "#
        )
        .bind(x_user_id)
        .fetch_optional(pool)
        .await
    }
}

#[allow(dead_code)]
impl AccountNft {
    pub async fn create(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        nft_object_id: &str,
        nft_type: &str,
        name: Option<&str>,
        image_url: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, AccountNft>(
            r#"
            INSERT INTO account_nfts (x_user_id, nft_object_id, nft_type, name, image_url)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, x_user_id, nft_object_id, nft_type, name, image_url, created_at, updated_at
            "#
        )
        .bind(x_user_id)
        .bind(nft_object_id)
        .bind(nft_type)
        .bind(name)
        .bind(image_url)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_x_user_id(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, AccountNft>(
            r#"
            SELECT id, x_user_id, nft_object_id, nft_type, name, image_url, created_at, updated_at
            FROM account_nfts
            WHERE x_user_id = $1
            "#
        )
        .bind(x_user_id)
        .fetch_all(pool)
        .await
    }

    pub async fn delete_by_nft_object_id(
        pool: &sqlx::PgPool,
        nft_object_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM account_nfts
            WHERE nft_object_id = $1
            "#
        )
        .bind(nft_object_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}


impl WebhookEvent {
    pub async fn create(
        pool: &sqlx::PgPool,
        event_id: &str,
        tweet_id: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, WebhookEvent>(
            r#"
            INSERT INTO webhook_events (event_id, tweet_id, payload)
            VALUES ($1, $2, $3)
            RETURNING id, event_id, tweet_id, payload, status, tx_digest, error_message, created_at, updated_at
            "#,
        )
        .bind(event_id)
        .bind(tweet_id)
        .bind(payload)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_event_id(
        pool: &sqlx::PgPool,
        event_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, WebhookEvent>(
            r#"
            SELECT id, event_id, tweet_id, payload, status, tx_digest, error_message, created_at, updated_at
            FROM webhook_events
            WHERE event_id = $1
            "#,
        )
        .bind(event_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn exists(pool: &sqlx::PgPool, event_id: &str) -> Result<bool, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct ExistsResult {
            exists: Option<bool>,
        }

        let result = sqlx::query_as::<_, ExistsResult>(
            r#"
            SELECT EXISTS(SELECT 1 FROM webhook_events WHERE event_id = $1) as exists
            "#,
        )
        .bind(event_id)
        .fetch_one(pool)
        .await?;

        Ok(result.exists.unwrap_or(false))
    }

    /// Update status to processing (đang xử lý)
    pub async fn set_processing(pool: &sqlx::PgPool, event_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'processing', updated_at = NOW()
            WHERE event_id = $1
            "#,
        )
        .bind(event_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update status to submitting (đang submit PTB)
    pub async fn set_submitting(pool: &sqlx::PgPool, event_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'submitting', updated_at = NOW()
            WHERE event_id = $1
            "#,
        )
        .bind(event_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update status to replying (submit xong, đang reply tweet)
    pub async fn set_replying(
        pool: &sqlx::PgPool,
        event_id: &str,
        tx_digest: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'replying', tx_digest = $2, updated_at = NOW()
            WHERE event_id = $1
            "#,
        )
        .bind(event_id)
        .bind(tx_digest)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update status to completed (hoàn tất)
    pub async fn set_completed(pool: &sqlx::PgPool, event_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'completed', updated_at = NOW()
            WHERE event_id = $1
            "#,
        )
        .bind(event_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update status to failed với error message
    pub async fn set_failed(
        pool: &sqlx::PgPool,
        event_id: &str,
        error_message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE webhook_events
            SET status = 'failed', error_message = $2, updated_at = NOW()
            WHERE event_id = $1
            "#,
        )
        .bind(event_id)
        .bind(error_message)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check if event is already completed or being processed
    pub fn is_done(&self) -> bool {
        matches!(self.status, EventStatus::Completed | EventStatus::Failed)
    }

    /// Find recent webhook events by x_user_id (pending/processing or last 24h)
    pub async fn find_recent_by_x_user_id(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, WebhookEvent>(
            r#"
            SELECT id, event_id, tweet_id, payload, status, tx_digest, error_message, created_at, updated_at
            FROM webhook_events
            WHERE payload->>'user_id' = $1
              AND (
                status NOT IN ('completed', 'failed')
                OR created_at > NOW() - INTERVAL '24 hours'
              )
            ORDER BY created_at DESC
            LIMIT 50
            "#,
        )
        .bind(x_user_id)
        .fetch_all(pool)
        .await
    }
}

/// Coin transfer type enum matching PostgreSQL coin_transfer_type type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "coin_transfer_type", rename_all = "snake_case")]
pub enum CoinTransferType {
    Transfer,
    Deposit,
    Withdraw,
}

/// NFT transfer type enum matching PostgreSQL nft_transfer_type type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "nft_transfer_type", rename_all = "snake_case")]
pub enum NftTransferType {
    Transfer,
    Deposit,
    Withdraw,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CoinTransfer {
    pub id: i32,
    pub tx_digest: String,
    pub transfer_type: CoinTransferType,
    pub from_id: Option<String>,    // x_user_id or external address
    pub to_id: Option<String>,      // x_user_id or external address
    pub coin_type: String,
    pub amount: i64,
    pub tweet_id: Option<String>,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NftTransfer {
    pub id: i32,
    pub tx_digest: String,
    pub transfer_type: NftTransferType,
    pub from_id: Option<String>,    // x_user_id or external address
    pub to_id: Option<String>,      // x_user_id or external address
    pub nft_object_id: String,
    pub nft_type: Option<String>,
    pub nft_name: Option<String>,
    pub tweet_id: Option<String>,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LinkWalletHistory {
    pub id: i32,
    pub tx_digest: String,
    pub x_user_id: String,
    pub from_address: String,       // old owner address
    pub to_address: String,         // new owner address
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
}

impl CoinTransfer {
    /// Find a single coin transfer by tx digest
    pub async fn find_by_digest(
        pool: &sqlx::PgPool,
        digest: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, CoinTransfer>(
            r#"
            SELECT id, tx_digest, transfer_type, from_id, to_id,
                   coin_type, amount, tweet_id, timestamp, created_at
            FROM coin_transfers
            WHERE tx_digest = $1
            "#,
        )
        .bind(digest)
        .fetch_optional(pool)
        .await
    }
}

impl NftTransfer {
    /// Find a single NFT transfer by tx digest
    pub async fn find_by_digest(
        pool: &sqlx::PgPool,
        digest: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, NftTransfer>(
            r#"
            SELECT id, tx_digest, transfer_type, from_id, to_id,
                   nft_object_id, nft_type, nft_name, tweet_id, timestamp, created_at
            FROM nft_transfers
            WHERE tx_digest = $1
            "#,
        )
        .bind(digest)
        .fetch_optional(pool)
        .await
    }
}

impl LinkWalletHistory {
    /// Find a single link wallet history by tx digest
    pub async fn find_by_digest(
        pool: &sqlx::PgPool,
        digest: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, LinkWalletHistory>(
            r#"
            SELECT id, tx_digest, x_user_id, from_address, to_address, timestamp, created_at
            FROM link_wallet_history
            WHERE tx_digest = $1
            "#,
        )
        .bind(digest)
        .fetch_optional(pool)
        .await
    }
}

// ====== Transaction Query ======

/// Unified transaction row from UNION query
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UnifiedTransaction {
    pub tx_digest: String,
    pub tx_type: String,
    pub from_id: Option<String>,
    pub to_id: Option<String>,
    pub coin_type: Option<String>,
    pub amount: Option<i64>,
    pub nft_object_id: Option<String>,
    pub nft_type: Option<String>,
    pub nft_name: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub tweet_id: Option<String>,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
}

impl UnifiedTransaction {
    /// Query all transactions for a user with SQL-level pagination using UNION
    pub async fn find_by_x_user_id_paginated(
        pool: &sqlx::PgPool,
        x_user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, UnifiedTransaction>(
            r#"
            SELECT * FROM (
                SELECT
                    tx_digest,
                    'coin_' || transfer_type::text as tx_type,
                    from_id,
                    to_id,
                    coin_type,
                    amount,
                    NULL::varchar as nft_object_id,
                    NULL::varchar as nft_type,
                    NULL::varchar as nft_name,
                    NULL::varchar as from_address,
                    NULL::varchar as to_address,
                    tweet_id,
                    timestamp,
                    created_at
                FROM coin_transfers
                WHERE from_id = $1 OR to_id = $1

                UNION ALL

                SELECT
                    tx_digest,
                    'nft_' || transfer_type::text as tx_type,
                    from_id,
                    to_id,
                    NULL::varchar as coin_type,
                    NULL::bigint as amount,
                    nft_object_id,
                    nft_type,
                    nft_name,
                    NULL::varchar as from_address,
                    NULL::varchar as to_address,
                    tweet_id,
                    timestamp,
                    created_at
                FROM nft_transfers
                WHERE from_id = $1 OR to_id = $1

                UNION ALL

                SELECT
                    tx_digest,
                    'link_wallet' as tx_type,
                    x_user_id as from_id,
                    NULL::varchar as to_id,
                    NULL::varchar as coin_type,
                    NULL::bigint as amount,
                    NULL::varchar as nft_object_id,
                    NULL::varchar as nft_type,
                    NULL::varchar as nft_name,
                    from_address,
                    to_address,
                    NULL::varchar as tweet_id,
                    timestamp,
                    created_at
                FROM link_wallet_history
                WHERE x_user_id = $1
            ) AS unified
            ORDER BY timestamp DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(x_user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Count total transactions for a user across all tables
    pub async fn count_by_x_user_id(
        pool: &sqlx::PgPool,
        x_user_id: &str,
    ) -> Result<i64, sqlx::Error> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM coin_transfers WHERE from_id = $1 OR to_id = $1) +
                (SELECT COUNT(*) FROM nft_transfers WHERE from_id = $1 OR to_id = $1) +
                (SELECT COUNT(*) FROM link_wallet_history WHERE x_user_id = $1) as total
            "#,
        )
        .bind(x_user_id)
        .fetch_one(pool)
        .await?;
        Ok(count.0)
    }
}

impl IndexerState {
    pub async fn get_by_name(pool: &sqlx::PgPool, name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, IndexerState>(
            r#"
            SELECT id, name, cursor, created_at, updated_at
            FROM indexer_state
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(pool)
        .await
    }

    pub async fn upsert_cursor(
        pool: &sqlx::PgPool,
        name: &str,
        cursor: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, IndexerState>(
            r#"
            INSERT INTO indexer_state (name, cursor)
            VALUES ($1, $2)
            ON CONFLICT (name)
            DO UPDATE SET cursor = EXCLUDED.cursor, updated_at = NOW()
            RETURNING id, name, cursor, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(cursor)
        .fetch_one(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== EventStatus tests ======

    #[test]
    fn test_event_status_serialization() {
        let status = EventStatus::Pending;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"Pending\"");
    }

    #[test]
    fn test_event_status_deserialization() {
        let json = "\"Processing\"";
        let status: EventStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, EventStatus::Processing);
    }

    #[test]
    fn test_event_status_all_variants_serialize() {
        let statuses = vec![
            EventStatus::Pending,
            EventStatus::Processing,
            EventStatus::Submitting,
            EventStatus::Replying,
            EventStatus::Completed,
            EventStatus::Failed,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: EventStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_event_status_equality() {
        assert_eq!(EventStatus::Pending, EventStatus::Pending);
        assert_ne!(EventStatus::Pending, EventStatus::Completed);
    }

    #[test]
    fn test_event_status_clone() {
        let status = EventStatus::Processing;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    // ====== Struct Debug trait tests ======

    #[test]
    fn test_event_status_debug() {
        let status = EventStatus::Completed;
        let debug = format!("{:?}", status);
        assert_eq!(debug, "Completed");
    }
}
