//! App configuration API

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::webhook::handler::AppState;

#[derive(Serialize)]
pub struct ConfigResponse {
    pub sponsor_enabled: bool,
}

/// GET /api/config - Get app configuration
pub async fn get_app_config(State(state): State<Arc<AppState>>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        sponsor_enabled: state.config.is_sponsor_enabled,
    })
}
