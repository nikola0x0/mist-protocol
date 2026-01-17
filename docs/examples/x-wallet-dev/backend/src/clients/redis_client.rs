use anyhow::Result;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{de::DeserializeOwned, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ==================== Rate Limiting ====================

/// Rate limit configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed
    pub max_requests: u64,
    /// Time window in seconds
    pub window_secs: u64,
}

impl RateLimitConfig {
    pub const fn new(max_requests: u64, window_secs: u64) -> Self {
        Self { max_requests, window_secs }
    }
}

/// Predefined rate limit configurations
pub mod rate_limits {
    use super::RateLimitConfig;

    /// Tweet commands: 5 per minute per user
    pub const TWEET_COMMAND: RateLimitConfig = RateLimitConfig::new(5, 60);
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed {
        /// Number of requests remaining in current window
        remaining: u64,
        /// Seconds until window resets
        #[allow(dead_code)]
        reset_in: u64,
    },
    /// Request is rate limited
    Limited {
        /// Seconds until rate limit resets
        retry_after: u64,
    },
}

#[allow(dead_code)]
impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }

    pub fn is_limited(&self) -> bool {
        matches!(self, RateLimitResult::Limited { .. })
    }
}

/// Cache TTL constants
pub mod cache_ttl {
    /// Coin metadata cache TTL: 24 hours (metadata rarely changes)
    pub const COIN_METADATA: u64 = 86400;
    /// Account cache TTL: 1 hour
    pub const ACCOUNT: u64 = 3600;
}

/// Cache key prefixes
pub mod cache_keys {
    pub const COIN_METADATA: &str = "coin_metadata";
    pub const ACCOUNT_BY_XID: &str = "account:xid";
    pub const ACCOUNT_BY_SUI_OBJECT: &str = "account:sui_object";
    pub const ACCOUNT_BY_OWNER: &str = "account:owner";
}

#[derive(Clone)]
pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self { manager })
    }

    pub async fn check_dedup(&self, key: &str) -> Result<bool> {
        let mut conn = self.manager.clone();
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    pub async fn set_dedup(&self, key: &str, ttl_seconds: u64) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.set_ex::<_, _, ()>(key, "1", ttl_seconds).await?;
        Ok(())
    }

    /// Legacy: Push to generic queue (use push_user_queue for per-user queues)
    #[allow(dead_code)]
    pub async fn push_queue(&self, queue_name: &str, value: &str) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.rpush::<_, _, ()>(queue_name, value).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn pop_queue(&self, queue_name: &str) -> Result<Option<String>> {
        let mut conn = self.manager.clone();
        let value: Option<String> = conn.lpop(queue_name, None).await?;
        Ok(value)
    }

    /// Legacy: Pop from generic queue (use pop_user_queue_blocking for per-user queues)
    #[allow(dead_code)]
    pub async fn pop_queue_blocking(
        &self,
        queue_name: &str,
        timeout_seconds: usize,
    ) -> Result<Option<String>> {
        let mut conn = self.manager.clone();
        // BLPOP returns (list, value)
        let result: Option<(String, String)> =
            conn.blpop(queue_name, timeout_seconds as f64).await?;
        Ok(result.map(|(_, value)| value))
    }

    pub async fn set_cache(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.set_ex::<_, _, ()>(key, value, ttl_seconds).await?;
        Ok(())
    }

    pub async fn get_cache(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.manager.clone();
        let value: Option<String> = conn.get(key).await?;
        Ok(value)
    }

    /// Set JSON-serializable value in cache
    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T, ttl_seconds: u64) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.set_cache(key, &json, ttl_seconds).await
    }

    /// Get JSON-deserializable value from cache
    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get_cache(key).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    /// Delete a cache key
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.manager.clone();
        conn.del::<_, ()>(key).await?;
        Ok(())
    }

    /// Delete multiple cache keys by pattern (use with caution)
    #[allow(dead_code)]
    pub async fn delete_pattern(&self, pattern: &str) -> Result<u64> {
        let mut conn = self.manager.clone();
        let keys: Vec<String> = conn.keys(pattern).await?;
        if keys.is_empty() {
            return Ok(0);
        }
        let count = keys.len() as u64;
        for key in keys {
            conn.del::<_, ()>(&key).await?;
        }
        Ok(count)
    }

    // ==================== Coin Metadata Cache ====================

    /// Get cached coin decimals
    pub async fn get_coin_decimals(&self, coin_type: &str) -> Result<Option<u8>> {
        let key = format!("{}:{}", cache_keys::COIN_METADATA, coin_type);
        self.get_json(&key).await
    }

    /// Set coin decimals in cache
    pub async fn set_coin_decimals(&self, coin_type: &str, decimals: u8) -> Result<()> {
        let key = format!("{}:{}", cache_keys::COIN_METADATA, coin_type);
        self.set_json(&key, &decimals, cache_ttl::COIN_METADATA).await
    }

    /// Get multiple coin decimals from cache (batch operation)
    pub async fn get_coin_decimals_batch(&self, coin_types: &[String]) -> Result<std::collections::HashMap<String, u8>> {
        let mut result = std::collections::HashMap::new();
        for coin_type in coin_types {
            if let Some(decimals) = self.get_coin_decimals(coin_type).await? {
                result.insert(coin_type.clone(), decimals);
            }
        }
        Ok(result)
    }

    // ==================== Account Cache ====================

    /// Invalidate all account caches for a given x_user_id
    #[allow(dead_code)]
    pub async fn invalidate_account_cache(&self, x_user_id: &str) -> Result<()> {
        let key = format!("{}:{}", cache_keys::ACCOUNT_BY_XID, x_user_id);
        self.delete(&key).await?;
        Ok(())
    }

    /// Invalidate account cache by sui_object_id
    #[allow(dead_code)]
    pub async fn invalidate_account_cache_by_sui_object(&self, sui_object_id: &str) -> Result<()> {
        let key = format!("{}:{}", cache_keys::ACCOUNT_BY_SUI_OBJECT, sui_object_id);
        self.delete(&key).await?;
        Ok(())
    }

    // ==================== Rate Limiting ====================

    /// Check rate limit for a user action using sliding window counter
    ///
    /// # Arguments
    /// * `user_id` - The user identifier (e.g., Twitter user ID)
    /// * `action` - The action being rate limited (e.g., "tweet_command", "transfer")
    /// * `config` - Rate limit configuration
    ///
    /// # Returns
    /// * `RateLimitResult::Allowed` if the request is within limits
    /// * `RateLimitResult::Limited` if the user has exceeded the rate limit
    pub async fn check_rate_limit(
        &self,
        user_id: &str,
        action: &str,
        config: &RateLimitConfig,
    ) -> Result<RateLimitResult> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate current window
        let window = now / config.window_secs;
        let reset_in = (window + 1) * config.window_secs - now;

        let key = format!("ratelimit:{}:{}:{}", action, user_id, window);

        let mut conn = self.manager.clone();

        // Increment counter and get new value
        let count: u64 = conn.incr(&key, 1).await?;

        // Set TTL on first request in this window
        if count == 1 {
            // TTL = window_secs + 1 to ensure cleanup after window expires
            conn.expire::<_, ()>(&key, (config.window_secs + 1) as i64).await?;
        }

        if count > config.max_requests {
            Ok(RateLimitResult::Limited {
                retry_after: reset_in,
            })
        } else {
            Ok(RateLimitResult::Allowed {
                remaining: config.max_requests - count,
                reset_in,
            })
        }
    }

    /// Check rate limit without incrementing the counter (peek)
    #[allow(dead_code)]
    pub async fn peek_rate_limit(
        &self,
        user_id: &str,
        action: &str,
        config: &RateLimitConfig,
    ) -> Result<RateLimitResult> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window = now / config.window_secs;
        let reset_in = (window + 1) * config.window_secs - now;

        let key = format!("ratelimit:{}:{}:{}", action, user_id, window);

        let mut conn = self.manager.clone();
        let count: u64 = conn.get(&key).await.unwrap_or(0);

        if count >= config.max_requests {
            Ok(RateLimitResult::Limited {
                retry_after: reset_in,
            })
        } else {
            Ok(RateLimitResult::Allowed {
                remaining: config.max_requests - count,
                reset_in,
            })
        }
    }

    /// Reset rate limit for a user action (admin use)
    #[allow(dead_code)]
    pub async fn reset_rate_limit(&self, user_id: &str, action: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Delete current and previous window keys
        for offset in 0..=1 {
            let window = now / 60 - offset; // Assume 60s window for cleanup
            let key = format!("ratelimit:{}:{}:{}", action, user_id, window);
            self.delete(&key).await?;
        }
        Ok(())
    }

    // ==================== Per-User Queue Management ====================

    /// Push item to a user-specific queue and register in active queues set
    #[allow(dead_code)]
    pub async fn push_user_queue(&self, xid: &str, value: &str) -> Result<()> {
        use crate::constants::redis::{queue_tweets_user, ACTIVE_QUEUES_SET};

        let queue_name = queue_tweets_user(xid);
        let mut conn = self.manager.clone();

        // Use pipeline for atomicity: RPUSH + SADD
        redis::pipe()
            .rpush(&queue_name, value)
            .sadd(ACTIVE_QUEUES_SET, xid)
            .query_async::<_, ()>(&mut conn)
            .await?;

        Ok(())
    }

    /// Pop from a specific user queue, remove from active set if empty
    #[allow(dead_code)]
    pub async fn pop_user_queue(&self, xid: &str) -> Result<Option<String>> {
        use crate::constants::redis::{queue_tweets_user, ACTIVE_QUEUES_SET};

        let queue_name = queue_tweets_user(xid);
        let mut conn = self.manager.clone();

        // Pop item
        let value: Option<String> = conn.lpop(&queue_name, None).await?;

        // If queue is now empty, remove from active set
        if value.is_some() {
            let queue_len: i64 = conn.llen(&queue_name).await?;
            if queue_len == 0 {
                conn.srem::<_, _, ()>(ACTIVE_QUEUES_SET, xid).await?;
            }
        }

        Ok(value)
    }

    /// Get all active user XIDs (users with pending items in their queues)
    pub async fn get_active_queue_users(&self) -> Result<Vec<String>> {
        use crate::constants::redis::ACTIVE_QUEUES_SET;

        let mut conn = self.manager.clone();
        let users: Vec<String> = conn.smembers(ACTIVE_QUEUES_SET).await?;
        Ok(users)
    }

    /// Pop from next available user queue (round-robin style)
    /// Returns (xid, value) if found, None if all queues empty
    #[allow(dead_code)]
    pub async fn pop_next_user_queue(&self, last_xid: Option<&str>) -> Result<Option<(String, String)>> {
        let users = self.get_active_queue_users().await?;

        if users.is_empty() {
            return Ok(None);
        }

        // Find starting index for round-robin
        let start_idx = match last_xid {
            Some(last) => {
                users.iter().position(|u| u == last)
                    .map(|i| (i + 1) % users.len())
                    .unwrap_or(0)
            }
            None => 0,
        };

        // Try each user queue in round-robin order
        for i in 0..users.len() {
            let idx = (start_idx + i) % users.len();
            let xid = &users[idx];

            if let Some(value) = self.pop_user_queue(xid).await? {
                return Ok(Some((xid.clone(), value)));
            }
        }

        Ok(None)
    }

    /// Blocking pop from any active user queue
    /// Uses BLPOP on multiple queues for efficiency
    #[allow(dead_code)]
    pub async fn pop_user_queue_blocking(&self, timeout_seconds: usize) -> Result<Option<(String, String)>> {
        use crate::constants::redis::{queue_tweets_user, ACTIVE_QUEUES_SET, QUEUE_TWEETS_USER_PREFIX};

        let users = self.get_active_queue_users().await?;

        if users.is_empty() {
            // No active queues, sleep briefly and return
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            return Ok(None);
        }

        // Build list of queue names
        let queue_names: Vec<String> = users.iter()
            .map(|xid| queue_tweets_user(xid))
            .collect();

        let mut conn = self.manager.clone();

        // BLPOP on multiple queues - returns (queue_name, value)
        let result: Option<(String, String)> = conn
            .blpop(&queue_names, timeout_seconds as f64)
            .await?;

        match result {
            Some((queue_name, value)) => {
                // Extract XID from queue name (queue:tweets:user:{xid})
                let xid = queue_name
                    .strip_prefix(QUEUE_TWEETS_USER_PREFIX)
                    .unwrap_or(&queue_name)
                    .to_string();

                // Check if queue is now empty, remove from active set
                let queue_len: i64 = conn.llen(&queue_name).await?;
                if queue_len == 0 {
                    conn.srem::<_, _, ()>(ACTIVE_QUEUES_SET, &xid).await?;
                }

                Ok(Some((xid, value)))
            }
            None => Ok(None),
        }
    }

    /// Get queue length for a specific user
    #[allow(dead_code)]
    pub async fn get_user_queue_length(&self, xid: &str) -> Result<i64> {
        use crate::constants::redis::queue_tweets_user;

        let queue_name = queue_tweets_user(xid);
        let mut conn = self.manager.clone();
        let len: i64 = conn.llen(&queue_name).await?;
        Ok(len)
    }

    // ==================== Per-User Sorted Queue (ordered by tweet time) ====================

    /// Push item to a user-specific sorted queue (ordered by tweet timestamp)
    /// Uses ZADD with tweet_timestamp_ms as score for chronological ordering
    pub async fn push_user_queue_sorted(&self, xid: &str, value: &str, tweet_timestamp_ms: u64) -> Result<()> {
        use crate::constants::redis::{sorted_queue_tweets_user, ACTIVE_SORTED_QUEUES_SET};

        let queue_name = sorted_queue_tweets_user(xid);
        let mut conn = self.manager.clone();

        // Use pipeline for atomicity: ZADD + SADD
        redis::pipe()
            .zadd(&queue_name, value, tweet_timestamp_ms as f64)
            .sadd(ACTIVE_SORTED_QUEUES_SET, xid)
            .query_async::<_, ()>(&mut conn)
            .await?;

        Ok(())
    }

    /// Pop oldest item from a user's sorted queue (by tweet timestamp)
    /// Returns (value, score) if found
    #[allow(dead_code)]
    pub async fn pop_user_queue_sorted(&self, xid: &str) -> Result<Option<(String, f64)>> {
        use crate::constants::redis::{sorted_queue_tweets_user, ACTIVE_SORTED_QUEUES_SET};

        let queue_name = sorted_queue_tweets_user(xid);
        let mut conn = self.manager.clone();

        // ZPOPMIN returns the item with the lowest score (oldest tweet)
        let result: Vec<(String, f64)> = conn.zpopmin(&queue_name, 1).await?;

        if let Some((value, score)) = result.into_iter().next() {
            // Check if queue is now empty, remove from active set
            let queue_len: i64 = conn.zcard(&queue_name).await?;
            if queue_len == 0 {
                conn.srem::<_, _, ()>(ACTIVE_SORTED_QUEUES_SET, xid).await?;
            }
            Ok(Some((value, score)))
        } else {
            Ok(None)
        }
    }

    /// Get all active user XIDs with sorted queues
    pub async fn get_active_sorted_queue_users(&self) -> Result<Vec<String>> {
        use crate::constants::redis::ACTIVE_SORTED_QUEUES_SET;

        let mut conn = self.manager.clone();
        let users: Vec<String> = conn.smembers(ACTIVE_SORTED_QUEUES_SET).await?;
        Ok(users)
    }

    /// Pop oldest item from any active user's sorted queue
    /// Iterates through users and returns the globally oldest tweet
    pub async fn pop_oldest_from_sorted_queues(&self) -> Result<Option<(String, String, u64)>> {
        use crate::constants::redis::{sorted_queue_tweets_user, ACTIVE_SORTED_QUEUES_SET};

        let users = self.get_active_sorted_queue_users().await?;

        if users.is_empty() {
            return Ok(None);
        }

        let mut conn = self.manager.clone();

        // Find the user with the oldest tweet (lowest score)
        let mut oldest: Option<(String, String, f64)> = None; // (xid, value, score)

        for xid in &users {
            let queue_name = sorted_queue_tweets_user(xid);
            // ZRANGE with WITHSCORES to peek at oldest without removing
            let result: Vec<(String, f64)> = redis::cmd("ZRANGE")
                .arg(&queue_name)
                .arg(0)
                .arg(0)
                .arg("WITHSCORES")
                .query_async(&mut conn)
                .await?;

            if let Some((value, score)) = result.into_iter().next() {
                match &oldest {
                    None => oldest = Some((xid.clone(), value, score)),
                    Some((_, _, old_score)) if score < *old_score => {
                        oldest = Some((xid.clone(), value, score));
                    }
                    _ => {}
                }
            }
        }

        // Now pop from the user with the oldest tweet
        if let Some((xid, _, _)) = oldest {
            let queue_name = sorted_queue_tweets_user(&xid);
            let result: Vec<(String, f64)> = conn.zpopmin(&queue_name, 1).await?;

            if let Some((value, score)) = result.into_iter().next() {
                // Check if queue is now empty
                let queue_len: i64 = conn.zcard(&queue_name).await?;
                if queue_len == 0 {
                    conn.srem::<_, _, ()>(ACTIVE_SORTED_QUEUES_SET, &xid).await?;
                }
                return Ok(Some((xid, value, score as u64)));
            }
        }

        Ok(None)
    }

    /// Pop oldest item with blocking behavior (polls periodically)
    pub async fn pop_oldest_from_sorted_queues_blocking(&self, timeout_seconds: usize) -> Result<Option<(String, String, u64)>> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_seconds as u64);

        loop {
            if let Some(result) = self.pop_oldest_from_sorted_queues().await? {
                return Ok(Some(result));
            }

            if start.elapsed() >= timeout {
                return Ok(None);
            }

            // Sleep briefly before retrying
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    // ==================== Distributed Locks ====================

    /// Distributed lock configuration
    pub const LOCK_TTL_SECS: u64 = 30; // Lock expires after 30 seconds
    pub const LOCK_RETRY_DELAY_MS: u64 = 100; // Retry every 100ms
    pub const LOCK_MAX_RETRIES: u32 = 50; // Max 5 seconds of waiting (50 * 100ms)

    /// Acquire a distributed lock for an account
    /// Returns a lock token if successful, None if lock couldn't be acquired
    pub async fn acquire_account_lock(&self, x_user_id: &str) -> Result<Option<String>> {
        let key = format!("lock:account:{}", x_user_id);
        let token = format!("{}:{}", x_user_id, SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis());

        let mut conn = self.manager.clone();

        // Use SET NX EX for atomic lock acquisition
        let result: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(&token)
            .arg("NX") // Only set if not exists
            .arg("EX") // Set expiration
            .arg(Self::LOCK_TTL_SECS)
            .query_async(&mut conn)
            .await?;

        if result.is_some() {
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// Acquire a distributed lock with retry
    /// Returns a lock token if successful within max retries
    pub async fn acquire_account_lock_with_retry(&self, x_user_id: &str) -> Result<Option<String>> {
        for _ in 0..Self::LOCK_MAX_RETRIES {
            if let Some(token) = self.acquire_account_lock(x_user_id).await? {
                return Ok(Some(token));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(Self::LOCK_RETRY_DELAY_MS)).await;
        }
        Ok(None)
    }

    /// Release a distributed lock (only if we own it)
    /// Uses Lua script to ensure atomic check-and-delete
    pub async fn release_account_lock(&self, x_user_id: &str, token: &str) -> Result<bool> {
        let key = format!("lock:account:{}", x_user_id);

        let mut conn = self.manager.clone();

        // Lua script: only delete if the token matches
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&key)
            .arg(token)
            .invoke_async(&mut conn)
            .await?;

        Ok(result == 1)
    }

    /// Extend lock TTL (for long-running operations)
    #[allow(dead_code)]
    pub async fn extend_account_lock(&self, x_user_id: &str, token: &str) -> Result<bool> {
        let key = format!("lock:account:{}", x_user_id);

        let mut conn = self.manager.clone();

        // Lua script: only extend if the token matches
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("expire", KEYS[1], ARGV[2])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&key)
            .arg(token)
            .arg(Self::LOCK_TTL_SECS)
            .invoke_async(&mut conn)
            .await?;

        Ok(result == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== RateLimitConfig tests ======

    #[test]
    fn test_rate_limit_config_new() {
        let config = RateLimitConfig::new(10, 60);
        assert_eq!(config.max_requests, 10);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_rate_limit_config_tweet_command() {
        let config = rate_limits::TWEET_COMMAND;
        assert_eq!(config.max_requests, 5);
        assert_eq!(config.window_secs, 60);
    }

    // ====== RateLimitResult tests ======

    #[test]
    fn test_rate_limit_result_allowed_is_allowed() {
        let result = RateLimitResult::Allowed {
            remaining: 5,
            reset_in: 30,
        };
        assert!(result.is_allowed());
        assert!(!result.is_limited());
    }

    #[test]
    fn test_rate_limit_result_limited_is_limited() {
        let result = RateLimitResult::Limited { retry_after: 30 };
        assert!(result.is_limited());
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_rate_limit_result_allowed_remaining() {
        let result = RateLimitResult::Allowed {
            remaining: 3,
            reset_in: 45,
        };

        if let RateLimitResult::Allowed { remaining, .. } = result {
            assert_eq!(remaining, 3);
        } else {
            panic!("Expected Allowed variant");
        }
    }

    #[test]
    fn test_rate_limit_result_limited_retry_after() {
        let result = RateLimitResult::Limited { retry_after: 60 };

        if let RateLimitResult::Limited { retry_after } = result {
            assert_eq!(retry_after, 60);
        } else {
            panic!("Expected Limited variant");
        }
    }

    // ====== Cache key format tests ======

    #[test]
    fn test_cache_key_coin_metadata() {
        assert_eq!(cache_keys::COIN_METADATA, "coin_metadata");
    }

    #[test]
    fn test_cache_key_account_by_xid() {
        assert_eq!(cache_keys::ACCOUNT_BY_XID, "account:xid");
    }

    #[test]
    fn test_cache_key_account_by_sui_object() {
        assert_eq!(cache_keys::ACCOUNT_BY_SUI_OBJECT, "account:sui_object");
    }

    #[test]
    fn test_cache_key_account_by_owner() {
        assert_eq!(cache_keys::ACCOUNT_BY_OWNER, "account:owner");
    }

    // ====== Cache TTL tests ======

    #[test]
    fn test_cache_ttl_coin_metadata() {
        assert_eq!(cache_ttl::COIN_METADATA, 86400); // 24 hours
    }

    #[test]
    fn test_cache_ttl_account() {
        assert_eq!(cache_ttl::ACCOUNT, 3600); // 1 hour
    }

    // ====== Rate limit key format test ======

    #[test]
    fn test_rate_limit_key_format() {
        // Verify the key format that would be generated
        let action = "tweet_command";
        let user_id = "123456";
        let window = 12345;

        let expected_key = format!("ratelimit:{}:{}:{}", action, user_id, window);
        assert_eq!(expected_key, "ratelimit:tweet_command:123456:12345");
    }

    #[test]
    fn test_rate_limit_window_calculation() {
        // Test window calculation logic
        let window_secs = 60u64;
        let timestamp = 1700000000u64; // Example timestamp

        let window = timestamp / window_secs;
        let reset_in = (window + 1) * window_secs - timestamp;

        // Window should be timestamp / 60
        assert_eq!(window, 28333333);
        // Reset should be within window_secs
        assert!(reset_in <= window_secs);
    }
}
