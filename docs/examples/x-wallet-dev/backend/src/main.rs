mod api;
mod clients;
mod config;
mod constants;
mod db;
mod error;
mod error_messages;
mod indexer;
mod processor;
mod services;
mod webhook;

use crate::clients::redis_client::RedisClient;
use crate::config::Config;
use crate::db::{create_pool, run_migrations};
use crate::indexer::Indexer;
use crate::processor::ProcessorWorker;
use crate::services::account_cache::AccountCacheService;
use crate::webhook::handler::{handle_crc_challenge, handle_webhook, handle_twitterapi_webhook, handle_poller_webhook, health_check, AppState};
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use axum::http::{header, HeaderValue, Method};
use tower_http::trace::TraceLayer;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer, key_extractor::PeerIpKeyExtractor};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "xwallet_backend=info,tower_http=debug".into()),
        )
        .init();

    info!("Starting X-Wallet Backend...");

    // Load config
    let config = Config::from_env()?;
    info!("Config loaded");

    // Setup database
    let db = create_pool(&config.database_url).await?;
    info!("Database connected");

    // Run migrations
    run_migrations(&db).await?;
    info!("Migrations completed");

    // Setup Redis
    let redis = RedisClient::new(&config.redis_url).await?;
    info!("Redis connected");

    // Create account cache service
    let account_cache = Arc::new(AccountCacheService::new(redis.clone(), db.clone()));
    info!("Account cache service initialized");

    // Create shared state
    let state = Arc::new(AppState {
        config: config.clone(),
        db,
        redis,
        account_cache,
    });

    // Start indexer worker (if enabled)
    if config.enable_indexer {
        info!("Indexer is ENABLED in API server");
        let indexer_config = config.clone();
        let indexer_db = state.db.clone();
        tokio::spawn(async move {
            match Indexer::new(indexer_config, indexer_db).await {
                Ok(indexer) => {
                    if let Err(e) = indexer.start().await {
                        tracing::error!("Indexer error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to initialize indexer: {}", e);
                }
            }
        });
    } else {
        info!("Indexer is DISABLED - run xwallet-indexer binary separately");
    }

    // Start transaction processor worker
    let processor_state = state.clone();
    tokio::spawn(async move {
        ProcessorWorker::new(processor_state).run().await;
    });

    // Build router
    let app = Router::new()
        .route("/", get(health_check))
        .route("/webhook", get(handle_crc_challenge).post(handle_webhook))
        .route("/webhook/twitterapi", axum::routing::post(handle_twitterapi_webhook))
        .route("/webhook/poller", axum::routing::post(handle_poller_webhook))
        .route(
            "/api/account/by-wallet/:address",
            get(crate::api::get_account_by_wallet),
        )
        .route(
            "/api/accounts/by-owner/:address",
            get(crate::api::get_account_by_owner),
        )
        .route(
            "/api/account/:sui_object_id/balance",
            get(crate::api::get_account_balance),
        )
        .route(
            "/api/account/:sui_object_id/nfts",
            get(crate::api::get_account_nfts),
        )
        .route(
            "/api/account/:sui_object_id/transactions",
            get(crate::api::get_transactions_by_account),
        )
        .route(
            "/api/tx/:digest",
            get(crate::api::get_transaction_by_digest),
        )
        .route("/api/accounts/search", get(crate::api::search_accounts))
        .route(
            "/api/accounts/:twitter_user_id",
            get(crate::api::get_account_by_twitter_id),
        )
        // Secured endpoint - requires access_token header
        .route("/api/my-account", get(crate::api::get_my_account))
        .route(
            "/api/link-wallet/generate-message",
            axum::routing::post(crate::api::generate_link_message),
        )
        .route(
            "/api/link-wallet/submit",
            axum::routing::post(crate::api::secure_link_wallet),
        )
        // Create account endpoint
        .route(
            "/api/account/create",
            axum::routing::post(crate::api::create_account),
        )
        // X OAuth 2.0 Authentication
        .route(
            "/api/auth/twitter/token",
            axum::routing::post(crate::api::exchange_twitter_token),
        )
        .route(
            "/api/auth/refresh",
            axum::routing::post(crate::api::refresh_token),
        )
        // Transaction Sponsorship (Enoki)
        .route(
            "/api/sponsor",
            axum::routing::post(crate::api::sponsor_transaction),
        )
        .route(
            "/api/execute",
            axum::routing::post(crate::api::execute_sponsored_transaction),
        )
        // Tweet status API (for Tweets tab)
        .route(
            "/api/account/:x_user_id/tweets",
            get(crate::api::get_account_tweets),
        )
        // App configuration
        .route("/api/config", get(crate::api::get_app_config))
        // Monitor check (from enclave-monitor)
        .route("/api/monitor/check", axum::routing::post(crate::api::handle_monitor_check))
        // Rate limiting: 100 requests per second per IP
        .layer({
            let governor_conf = Box::new(
                GovernorConfigBuilder::default()
                    .per_second(100)
                    .burst_size(200)
                    .key_extractor(PeerIpKeyExtractor)
                    .finish()
                    .unwrap(),
            );
            GovernorLayer {
                config: Box::leak(governor_conf),
            }
        })
        // Request timeout: 30 seconds
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer({
            let origins: Vec<HeaderValue> = config
                .cors_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();

            info!("CORS allowed origins: {:?}", config.cors_origins);

            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers([
                    header::CONTENT_TYPE,
                    header::AUTHORIZATION,
                    header::ACCEPT,
                ])
                .allow_credentials(true)
        })
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);

    // Check if ngrok is enabled via feature and NGROK_AUTHTOKEN is set
    #[cfg(feature = "ngrok")]
    if std::env::var("NGROK_AUTHTOKEN").is_ok() {
        use ngrok::prelude::*;

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("Starting with ngrok tunnel...");

        // Start local listener first
        let local_addr = format!("127.0.0.1:{}", config.port);
        let listener = tokio::net::TcpListener::bind(&local_addr).await?;
        let local_url = format!("http://{}", local_addr);

        // Create ngrok tunnel that forwards to local server
        let tunnel = ngrok::Session::builder()
            .authtoken_from_env()
            .connect()
            .await?
            .http_endpoint()
            .listen_and_forward(local_url.parse()?)
            .await?;

        let ngrok_url = tunnel.url().to_string();
        info!("🚀 ngrok URL: {}", ngrok_url);
        info!("📡 Webhook endpoint: {}/webhook/twitterapi", ngrok_url);
        info!("💚 Health check: {}/", ngrok_url);
        info!("🏠 Local: http://{}", local_addr);
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // Serve on local listener (ngrok forwards to it)
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
        return Ok(());
    }

    // Default: regular TCP listener
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Listening on http://{}", addr);
    info!("Webhook endpoint: http://{}/webhook", addr);
    info!("Health check: http://{}/", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
