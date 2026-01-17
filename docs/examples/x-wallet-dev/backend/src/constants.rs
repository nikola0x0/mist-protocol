// Redis keys
pub mod redis {
    /// Legacy: Single queue for tweet processing (deprecated, use per-user queues)
    #[allow(dead_code)]
    pub const QUEUE_TWEETS: &str = "queue:tweets";

    /// Per-user queue prefix for tweet processing
    pub const QUEUE_TWEETS_USER_PREFIX: &str = "queue:tweets:user:";

    /// Set of active user queues (for worker discovery)
    pub const ACTIVE_QUEUES_SET: &str = "active_queues";

    /// Per-user sorted queue prefix (ordered by tweet timestamp)
    pub const SORTED_QUEUE_TWEETS_USER_PREFIX: &str = "sorted_queue:tweets:user:";

    /// Set of active user sorted queues (for worker discovery)
    pub const ACTIVE_SORTED_QUEUES_SET: &str = "active_sorted_queues";

    /// Generate per-user queue name
    pub fn queue_tweets_user(xid: &str) -> String {
        format!("{}{}", QUEUE_TWEETS_USER_PREFIX, xid)
    }

    /// Generate per-user sorted queue name
    pub fn sorted_queue_tweets_user(xid: &str) -> String {
        format!("{}{}", SORTED_QUEUE_TWEETS_USER_PREFIX, xid)
    }

    /// Deduplication key prefix for tweets
    pub fn dedup_tweet(tweet_id: &str) -> String {
        format!("dedup:tweet:{}", tweet_id)
    }

    /// Deduplication key prefix for webhook events
    #[allow(dead_code)]
    pub fn dedup_webhook(event_id: &str) -> String {
        format!("dedup:webhook:{}", event_id)
    }

    /// Cache key prefix for account lookups
    #[allow(dead_code)]
    pub fn cache_account(xid: &str) -> String {
        format!("cache:account:{}", xid)
    }

    /// Rate limiting key prefix
    #[allow(dead_code)]
    pub fn ratelimit_user(user_id: &str) -> String {
        format!("ratelimit:user:{}", user_id)
    }

    /// TTL values in seconds
    pub const TTL_DEDUP: u64 = 86400; // 24 hours
    #[allow(dead_code)]
    pub const TTL_CACHE: u64 = 3600; // 1 hour
}

// Event ID formats
pub mod events {
    pub fn tweet_event_id(tweet_id: &str) -> String {
        format!("tweet:{}", tweet_id)
    }
}

// Database
pub mod db {
    #[allow(dead_code)]
    pub const MAX_CONNECTIONS: u32 = 10;
}

// Server
pub mod server {
    #[allow(dead_code)]
    pub const DEFAULT_PORT: u16 = 3001;
    #[allow(dead_code)]
    pub const SHUTDOWN_TIMEOUT_SECS: u64 = 30;
}

// Twitter
pub mod twitter {
    #[allow(dead_code)]
    pub const API_BASE_URL: &str = "https://api.twitter.com/2";
}

// Sui
pub mod sui {
    #[allow(dead_code)]
    pub const TESTNET_RPC: &str = "https://fullnode.testnet.sui.io:443";
    #[allow(dead_code)]
    pub const MAINNET_RPC: &str = "https://fullnode.mainnet.sui.io:443";
}

// Coin utilities for amount formatting
pub mod coin {
    /// Coin information (symbol and decimals)
    #[derive(Debug, Clone)]
    pub struct CoinInfo {
        pub symbol: &'static str,
        pub decimals: u8,
    }

    /// Known coin configurations
    pub const SUI: CoinInfo = CoinInfo { symbol: "SUI", decimals: 9 };
    pub const USDC: CoinInfo = CoinInfo { symbol: "USDC", decimals: 6 };
    pub const WAL: CoinInfo = CoinInfo { symbol: "WAL", decimals: 9 };
    pub const DEFAULT_DECIMALS: u8 = 9;

    /// Get coin info from coin type string (case-insensitive)
    pub fn get_coin_info(coin_type: &str) -> CoinInfo {
        let coin_type_lower = coin_type.to_lowercase();

        if coin_type_lower.contains("sui::sui") || coin_type_lower == "sui" {
            SUI
        } else if coin_type_lower.contains("usdc") {
            USDC
        } else if coin_type_lower.contains("wal::wal") || coin_type_lower == "wal" {
            WAL
        } else {
            // Extract symbol from type path for unknown coins
            let symbol = coin_type.split("::").last().unwrap_or("TOKEN");
            CoinInfo {
                symbol: Box::leak(symbol.to_string().into_boxed_str()),
                decimals: DEFAULT_DECIMALS,
            }
        }
    }

    /// Format raw amount to human-readable string
    /// Trims trailing zeros and decimal point
    pub fn format_amount(amount: u64, decimals: u8) -> String {
        let divisor = 10_u64.pow(decimals as u32);
        format!("{:.precision$}", amount as f64 / divisor as f64, precision = decimals as usize)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }

    /// Format amount with symbol (e.g., "1.5 SUI")
    pub fn format_amount_with_symbol(amount: u64, coin_type: &str) -> String {
        let info = get_coin_info(coin_type);
        format!("{} {}", format_amount(amount, info.decimals), info.symbol)
    }
}

// Enclave endpoints
pub mod enclave {
    #[allow(dead_code)]
    pub const DEFAULT_URL: &str = "http://localhost:3000";

    /// Unified tweet processing endpoint (handles all tweet-based commands)
    pub const PROCESS_TWEET_ENDPOINT: &str = "/process_tweet";

    /// For auto-creating recipient accounts (not tweet-based)
    pub const PROCESS_INIT_ACCOUNT_ENDPOINT: &str = "/process_init_account";

    /// For dApp update handle (not tweet-based)
    #[allow(dead_code)]
    pub const PROCESS_UPDATE_HANDLE_ENDPOINT: &str = "/process_update_handle";

    /// For dApp wallet linking (not tweet-based)
    pub const PROCESS_SECURE_LINK_WALLET_ENDPOINT: &str = "/process_secure_link_wallet";

    /// Health check endpoint
    #[allow(dead_code)]
    pub const HEALTH_CHECK_ENDPOINT: &str = "/health_check";

    /// Get attestation endpoint
    #[allow(dead_code)]
    pub const GET_ATTESTATION_ENDPOINT: &str = "/get_attestation";
}
