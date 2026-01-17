use anyhow::{Context, Result};
use tracing::debug;

use crate::clients::sui_client::{EventPage, SuiClient, XWALLET_MODULE};
use crate::config::Config;

pub struct EventFetcher {
    client: SuiClient,
    package_id: String,
}

impl EventFetcher {
    pub async fn new(config: Config) -> Result<Self> {
        let client = SuiClient::new(config.sui_rpc_url);

        Ok(Self {
            client,
            package_id: config.xwallet_package_id,
        })
    }

    /// Fetch events using cursor-based pagination
    pub async fn fetch_events(&self, cursor: Option<&str>, limit: u64) -> Result<EventPage> {
        debug!(
            "Fetching events with cursor: {:?}, limit: {}",
            cursor, limit
        );

        // Query events from Sui using the existing client
        let page = self
            .client
            .query_events(&self.package_id, XWALLET_MODULE, cursor, limit)
            .await
            .context("Failed to query events from Sui")?;

        debug!(
            "Fetched {} events, has_next_page: {}",
            page.data.len(),
            page.has_next_page
        );

        Ok(page)
    }
}
