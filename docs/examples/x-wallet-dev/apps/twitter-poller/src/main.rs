mod backend_client;
mod config;
mod poller;
mod twitter_adapter;

use anyhow::Result;
use config::Config;
use poller::PollerService;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "twitter_poller=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;

    // Create and start poller service
    let service = PollerService::new(config)?;
    service.start().await?;

    Ok(())
}
