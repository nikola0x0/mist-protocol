pub mod cursor;
pub mod event_fetcher;
pub mod event_processor;
pub mod handlers;
pub mod types;

use anyhow::Result;
use tokio::time::{interval, Duration};
use tracing::info;

use crate::config::Config;
use cursor::CursorManager;
use event_fetcher::EventFetcher;
use event_processor::EventProcessor;
use types::EventPage;

pub struct Indexer {
    event_fetcher: EventFetcher,
    event_processor: EventProcessor,
    cursor_manager: CursorManager,
    poll_interval: Duration,
}

impl Indexer {
    pub async fn new(config: Config, pool: sqlx::PgPool) -> Result<Self> {
        let event_fetcher = EventFetcher::new(config.clone()).await?;
        let event_processor = EventProcessor::new(pool.clone());
        let cursor_manager = CursorManager::new(pool.clone());
        let poll_interval = Duration::from_millis(config.indexer_poll_interval_ms);

        Ok(Self {
            event_fetcher,
            event_processor,
            cursor_manager,
            poll_interval,
        })
    }

    /// Start the indexer in real-time mode
    pub async fn start(&self) -> Result<()> {
        info!("Starting XWallet Indexer");

        // Load last cursor from database
        let mut cursor = self.cursor_manager.load_cursor("xwallet_events").await?;
        info!("Starting from cursor: {:?}", cursor);

        let mut ticker = interval(self.poll_interval);

        loop {
            ticker.tick().await;

            match self.fetch_and_process_events(&mut cursor).await {
                Ok(processed) => {
                    if processed > 0 {
                        info!("Processed {} events", processed);
                        // Save cursor after processing
                        self.cursor_manager
                            .save_cursor("xwallet_events", cursor.as_ref())
                            .await?;
                    }
                }
                Err(e) => {
                    tracing::error!("Error processing events: {}", e);
                    // Continue processing on next tick
                }
            }
        }
    }

    async fn fetch_and_process_events(&self, cursor: &mut Option<String>) -> Result<usize> {
        // Fetch events
        let page: EventPage = self
            .event_fetcher
            .fetch_events(cursor.as_deref(), 100)
            .await?;

        if page.data.is_empty() {
            return Ok(0);
        }

        info!("Fetched {} events", page.data.len());

        // Process events
        let processed = self.event_processor.process_events(&page.data).await?;

        // Update cursor from last event
        *cursor = page.next_cursor.map(|c| c.to_cursor());

        Ok(processed)
    }

    /// Sync all historical events from genesis
    #[allow(dead_code)]
    pub async fn sync_historical(&self) -> Result<()> {
        info!("Starting historical sync");

        let mut cursor: Option<String> = None;
        let mut total_processed = 0;

        loop {
            let page = self
                .event_fetcher
                .fetch_events(cursor.as_deref(), 1000)
                .await?;

            if page.data.is_empty() {
                break;
            }

            let processed = self.event_processor.process_events(&page.data).await?;
            total_processed += processed;

            info!(
                "Historical sync progress: {} events processed",
                total_processed
            );

            // Update cursor
            cursor = page.next_cursor.map(|c| c.to_cursor());

            // Save checkpoint periodically
            self.cursor_manager
                .save_cursor("xwallet_events", cursor.as_ref())
                .await?;

            if page.has_next_page {
                tokio::time::sleep(Duration::from_millis(100)).await;
            } else {
                break;
            }
        }

        info!(
            "Historical sync complete: {} events processed",
            total_processed
        );
        Ok(())
    }
}
