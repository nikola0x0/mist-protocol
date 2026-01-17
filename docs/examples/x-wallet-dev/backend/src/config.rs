use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    // Server
    pub port: u16,
    pub log_level: String,
    pub cors_origins: Vec<String>,

    // Database
    pub database_url: String,

    // Redis
    pub redis_url: String,

    // Twitter API
    pub twitter_api_key: String,
    pub twitter_api_secret: String,
    pub twitter_bearer_token: String,
    pub twitter_access_token: String,
    pub twitter_access_token_secret: String,
    pub twitter_bot_username: Option<String>,

    // Twitter OAuth 2.0 (for user authentication)
    pub twitter_oauth2_client_id: String,
    pub twitter_oauth2_client_secret: String,
    pub twitter_oauth2_redirect_uri: String,

    // Sui
    pub sui_rpc_url: String,
    pub xwallet_package_id: String,
    pub xwallet_registry_id: String,
    pub enclave_config_id: String,
    pub enclave_object_id: String,

    // Enoki (gas sponsorship)
    pub enoki_api_key: String,
    pub enoki_network: String,

    // Backend signer
    pub backend_signer_private_key: String,

    // Enclave
    pub enclave_url: String,

    // Indexer
    pub indexer_poll_interval_ms: u64,
    pub indexer_batch_size: u64,
    pub enable_indexer: bool,

    // Bot handles (for filtering valid mentions in webhooks)
    pub bot_handles: Vec<String>,

    // Slack notifications (optional)
    pub slack_webhook_url: Option<String>,

    // Frontend URL (for reply messages)
    pub frontend_url: String,

    // Poller API key (for authenticating twitter-poller service)
    pub poller_api_key: Option<String>,

    // Sponsor dApp transactions (gas sponsorship toggle)
    pub is_sponsor_enabled: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            // Server
            port: env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .context("PORT must be a valid u16")?,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173,http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),

            // Database
            database_url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,

            // Redis
            redis_url: env::var("REDIS_URL").context("REDIS_URL must be set")?,

            // Twitter API
            twitter_api_key: env::var("TWITTER_API_KEY").context("TWITTER_API_KEY must be set")?,
            twitter_api_secret: env::var("TWITTER_API_SECRET")
                .context("TWITTER_API_SECRET must be set")?,
            twitter_bearer_token: env::var("TWITTER_BEARER_TOKEN")
                .context("TWITTER_BEARER_TOKEN must be set")?,
            twitter_access_token: env::var("TWITTER_ACCESS_TOKEN")
                .context("TWITTER_ACCESS_TOKEN must be set")?,
            twitter_access_token_secret: env::var("TWITTER_ACCESS_TOKEN_SECRET")
                .context("TWITTER_ACCESS_TOKEN_SECRET must be set")?,
            twitter_bot_username: env::var("TWITTER_BOT_USERNAME").ok(),

            // Twitter OAuth 2.0
            twitter_oauth2_client_id: env::var("TWITTER_OAUTH2_CLIENT_ID")
                .context("TWITTER_OAUTH2_CLIENT_ID must be set")?,
            twitter_oauth2_client_secret: env::var("TWITTER_OAUTH2_CLIENT_SECRET")
                .context("TWITTER_OAUTH2_CLIENT_SECRET must be set")?,
            twitter_oauth2_redirect_uri: env::var("TWITTER_OAUTH2_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:5173/callback".to_string()),

            // Sui
            sui_rpc_url: env::var("SUI_RPC_URL")
                .unwrap_or_else(|_| "https://fullnode.testnet.sui.io:443".to_string()),
            xwallet_package_id: env::var("XWALLET_PACKAGE_ID")
                .context("XWALLET_PACKAGE_ID must be set")?,
            xwallet_registry_id: env::var("XWALLET_REGISTRY_ID")
                .context("XWALLET_REGISTRY_ID must be set")?,
            enclave_config_id: env::var("ENCLAVE_CONFIG_ID")
                .context("ENCLAVE_CONFIG_ID must be set")?,
            enclave_object_id: env::var("ENCLAVE_ID")
                .or_else(|_| env::var("ENCLAVE_OBJECT_ID"))
                .context("ENCLAVE_ID or ENCLAVE_OBJECT_ID must be set to the enclave shared object (NOT the config object)")?,

            // Enoki
            enoki_api_key: env::var("ENOKI_API_KEY").context("ENOKI_API_KEY must be set")?,
            enoki_network: env::var("ENOKI_NETWORK").unwrap_or_else(|_| "testnet".to_string()),

            // Backend signer
            backend_signer_private_key: env::var("BACKEND_SIGNER_PRIVATE_KEY")
                .context("BACKEND_SIGNER_PRIVATE_KEY must be set")?,

            // Enclave
            enclave_url: env::var("ENCLAVE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),

            // Indexer
            indexer_poll_interval_ms: env::var("INDEXER_POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            indexer_batch_size: env::var("INDEXER_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            enable_indexer: env::var("ENABLE_INDEXER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false), // Default: disabled in API server

            // Bot handles - comma-separated list of valid bot handles (without @)
            // Example: BOT_HANDLES=nautilusxwallet
            bot_handles: env::var("BOT_HANDLES")
                .unwrap_or_else(|_| "nautilusxwallet".to_string())
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),

            // Slack notifications (optional)
            slack_webhook_url: env::var("SLACK_WEBHOOK_URL").ok(),

            // Frontend URL (for reply messages)
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "https://xwallet.nautilus.sh".to_string()),

            // Poller API key
            poller_api_key: env::var("POLLER_API_KEY").ok(),

            // Sponsor toggle (default: true)
            is_sponsor_enabled: env::var("IS_SPONSOR_ENABLED")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(true),
        })
    }
}
