use anyhow::{Context, Result};
use sqlx::PgPool;
use tracing::debug;

use crate::db::models::IndexerState;

pub struct CursorManager {
    pool: PgPool,
}

impl CursorManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load cursor from database
    pub async fn load_cursor(&self, name: &str) -> Result<Option<String>> {
        let state = IndexerState::get_by_name(&self.pool, name)
            .await
            .context("Failed to load cursor from database")?;

        match state {
            Some(s) => {
                debug!("Loaded cursor for {}: {:?}", name, s.cursor);
                Ok(s.cursor)
            }
            None => {
                debug!("No cursor found for {}, starting from genesis", name);
                Ok(None)
            }
        }
    }

    /// Save cursor to database
    pub async fn save_cursor(&self, name: &str, cursor: Option<&String>) -> Result<()> {
        let cursor_str = cursor.map(|s| s.as_str());

        IndexerState::upsert_cursor(&self.pool, name, cursor_str)
            .await
            .context("Failed to save cursor to database")?;

        debug!("Saved cursor for {}: {:?}", name, cursor);
        Ok(())
    }

    /// Reset cursor (start from genesis)
    #[allow(dead_code)]
    pub async fn reset_cursor(&self, name: &str) -> Result<()> {
        IndexerState::upsert_cursor(&self.pool, name, None)
            .await
            .context("Failed to reset cursor")?;

        debug!("Reset cursor for {}", name);
        Ok(())
    }
}
