use crate::clients::redis_client::{cache_keys, cache_ttl, RedisClient};
use crate::db::models::XWalletAccount;
use anyhow::Result;
use sqlx::PgPool;

/// Account cache service that provides caching layer for account lookups
pub struct AccountCacheService {
    redis: RedisClient,
    db: PgPool,
}

impl AccountCacheService {
    pub fn new(redis: RedisClient, db: PgPool) -> Self {
        Self { redis, db }
    }

    /// Find account by x_user_id with caching
    pub async fn find_by_x_user_id(&self, x_user_id: &str) -> Result<Option<XWalletAccount>> {
        let cache_key = format!("{}:{}", cache_keys::ACCOUNT_BY_XID, x_user_id);

        // Try cache first
        if let Some(account) = self.redis.get_json::<XWalletAccount>(&cache_key).await? {
            return Ok(Some(account));
        }

        // Cache miss - fetch from DB
        let account = XWalletAccount::find_by_x_user_id(&self.db, x_user_id).await?;

        // Cache the result if found
        if let Some(ref acc) = account {
            if let Err(e) = self.redis.set_json(&cache_key, acc, cache_ttl::ACCOUNT).await {
                tracing::warn!("Failed to cache account {}: {:?}", x_user_id, e);
            }
        }

        Ok(account)
    }

    /// Find account by sui_object_id with caching
    pub async fn find_by_sui_object_id(&self, sui_object_id: &str) -> Result<Option<XWalletAccount>> {
        let cache_key = format!("{}:{}", cache_keys::ACCOUNT_BY_SUI_OBJECT, sui_object_id);

        // Try cache first
        if let Some(account) = self.redis.get_json::<XWalletAccount>(&cache_key).await? {
            return Ok(Some(account));
        }

        // Cache miss - fetch from DB
        let account = XWalletAccount::find_by_sui_object_id(&self.db, sui_object_id).await?;

        // Cache the result if found
        if let Some(ref acc) = account {
            if let Err(e) = self.redis.set_json(&cache_key, acc, cache_ttl::ACCOUNT).await {
                tracing::warn!("Failed to cache account by sui_object_id {}: {:?}", sui_object_id, e);
            }
            // Also cache by x_user_id for cross-lookup
            let xid_cache_key = format!("{}:{}", cache_keys::ACCOUNT_BY_XID, acc.x_user_id);
            let _ = self.redis.set_json(&xid_cache_key, acc, cache_ttl::ACCOUNT).await;
        }

        Ok(account)
    }

    /// Find account by owner_address with caching
    pub async fn find_by_owner_address(&self, owner_address: &str) -> Result<Option<XWalletAccount>> {
        let cache_key = format!("{}:{}", cache_keys::ACCOUNT_BY_OWNER, owner_address);

        // Try cache first
        if let Some(account) = self.redis.get_json::<XWalletAccount>(&cache_key).await? {
            return Ok(Some(account));
        }

        // Cache miss - fetch from DB
        let account = XWalletAccount::find_by_owner_address(&self.db, owner_address).await?;

        // Cache the result if found
        if let Some(ref acc) = account {
            if let Err(e) = self.redis.set_json(&cache_key, acc, cache_ttl::ACCOUNT).await {
                tracing::warn!("Failed to cache account by owner {}: {:?}", owner_address, e);
            }
            // Also cache by x_user_id and sui_object_id
            let xid_cache_key = format!("{}:{}", cache_keys::ACCOUNT_BY_XID, acc.x_user_id);
            let _ = self.redis.set_json(&xid_cache_key, acc, cache_ttl::ACCOUNT).await;
            let sui_cache_key = format!("{}:{}", cache_keys::ACCOUNT_BY_SUI_OBJECT, acc.sui_object_id);
            let _ = self.redis.set_json(&sui_cache_key, acc, cache_ttl::ACCOUNT).await;
        }

        Ok(account)
    }

    /// Invalidate all caches for an account
    #[allow(dead_code)]
    pub async fn invalidate(&self, account: &XWalletAccount) -> Result<()> {
        let xid_key = format!("{}:{}", cache_keys::ACCOUNT_BY_XID, account.x_user_id);
        let sui_key = format!("{}:{}", cache_keys::ACCOUNT_BY_SUI_OBJECT, account.sui_object_id);

        self.redis.delete(&xid_key).await?;
        self.redis.delete(&sui_key).await?;

        if let Some(ref owner) = account.owner_address {
            let owner_key = format!("{}:{}", cache_keys::ACCOUNT_BY_OWNER, owner);
            self.redis.delete(&owner_key).await?;
        }

        Ok(())
    }

    /// Invalidate cache by x_user_id (when you don't have full account)
    #[allow(dead_code)]
    pub async fn invalidate_by_x_user_id(&self, x_user_id: &str) -> Result<()> {
        // Fetch account to get all keys
        if let Some(account) = XWalletAccount::find_by_x_user_id(&self.db, x_user_id).await? {
            self.invalidate(&account).await?;
        } else {
            // Just invalidate xid key
            let xid_key = format!("{}:{}", cache_keys::ACCOUNT_BY_XID, x_user_id);
            self.redis.delete(&xid_key).await?;
        }
        Ok(())
    }
}
