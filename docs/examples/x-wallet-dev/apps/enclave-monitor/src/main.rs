mod config;
mod monitor;

use anyhow::Result;
use config::Config;
use monitor::MonitorService;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "enclave_monitor=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!(r#"
    ╔═══════════════════════════════════════════════╗
    ║         XWallet Enclave Monitor               ║
    ║      Health check + Slack notifications       ║
    ╚═══════════════════════════════════════════════╝
    "#);

    // Load configuration
    let config = Config::from_env()?;

    // Create and start monitor service
    let service = MonitorService::new(config)?;
    service.start().await?;

    Ok(())
}
