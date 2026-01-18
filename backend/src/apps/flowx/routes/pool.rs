use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::apps::flowx::services::FlowXService;

/// Request để tạo pool mới
#[derive(Debug, Deserialize)]
pub struct CreatePoolRequest {
    /// Fee rate: 500 (0.05%), 3000 (0.3%), 10000 (1%)
    pub fee_rate: u64,
    /// Giá khởi tạo: 1 SUI = initial_price YOUR_TOKEN
    pub initial_price: f64,
}

/// Response khi tạo pool
#[derive(Debug, Serialize)]
pub struct CreatePoolResponse {
    pub success: bool,
    pub tx_digest: Option<String>,
    pub message: String,
}

/// Request để thêm thanh khoản
#[derive(Debug, Deserialize)]
pub struct AddLiquidityRequest {
    /// Số lượng SUI (đơn vị nhỏ nhất, 9 decimals)
    pub amount_sui: u64,
    /// Số lượng YOUR_TOKEN (đơn vị nhỏ nhất)
    pub amount_token: u64,
    /// Giá thấp nhất của range
    pub price_lower: f64,
    /// Giá cao nhất của range
    pub price_upper: f64,
    /// Fee tier
    pub fee_rate: u64,
}

/// Response khi thêm thanh khoản
#[derive(Debug, Serialize)]
pub struct AddLiquidityResponse {
    pub success: bool,
    pub tx_digest: Option<String>,
    pub position_id: Option<String>,
    pub message: String,
}

/// Tạo pool mới
pub async fn create_pool(
    State(service): State<Arc<FlowXService>>,
    Json(req): Json<CreatePoolRequest>,
) -> impl IntoResponse {
    tracing::info!("Creating pool with fee_rate={}, initial_price={}", req.fee_rate, req.initial_price);
    
    // Validate fee rate
    if ![100, 500, 3000, 10000].contains(&req.fee_rate) {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreatePoolResponse {
                success: false,
                tx_digest: None,
                message: "Invalid fee rate. Must be 100, 500, 3000, or 10000".to_string(),
            }),
        );
    }
    
    if req.initial_price <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(CreatePoolResponse {
                success: false,
                tx_digest: None,
                message: "Initial price must be positive".to_string(),
            }),
        );
    }
    
    match service.create_pool(req.fee_rate, req.initial_price).await {
        Ok(tx_digest) => (
            StatusCode::OK,
            Json(CreatePoolResponse {
                success: true,
                tx_digest: Some(tx_digest),
                message: "Pool created successfully".to_string(),
            }),
        ),
        Err(e) => {
            tracing::error!("Failed to create pool: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreatePoolResponse {
                    success: false,
                    tx_digest: None,
                    message: format!("Failed to create pool: {}", e),
                }),
            )
        }
    }
}

/// Thêm thanh khoản
pub async fn add_liquidity(
    State(service): State<Arc<FlowXService>>,
    Json(req): Json<AddLiquidityRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Adding liquidity: {} SUI, {} TOKEN, range [{}, {}]",
        req.amount_sui,
        req.amount_token,
        req.price_lower,
        req.price_upper
    );
    
    // Validate
    if req.price_lower >= req.price_upper {
        return (
            StatusCode::BAD_REQUEST,
            Json(AddLiquidityResponse {
                success: false,
                tx_digest: None,
                position_id: None,
                message: "price_lower must be less than price_upper".to_string(),
            }),
        );
    }
    
    if req.amount_sui == 0 && req.amount_token == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(AddLiquidityResponse {
                success: false,
                tx_digest: None,
                position_id: None,
                message: "At least one amount must be non-zero".to_string(),
            }),
        );
    }
    
    match service
        .add_liquidity(
            req.amount_sui,
            req.amount_token,
            req.price_lower,
            req.price_upper,
            req.fee_rate,
        )
        .await
    {
        Ok(tx_digest) => (
            StatusCode::OK,
            Json(AddLiquidityResponse {
                success: true,
                tx_digest: Some(tx_digest),
                position_id: None, // TODO: Extract from transaction effects
                message: "Liquidity added successfully".to_string(),
            }),
        ),
        Err(e) => {
            tracing::error!("Failed to add liquidity: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AddLiquidityResponse {
                    success: false,
                    tx_digest: None,
                    position_id: None,
                    message: format!("Failed to add liquidity: {}", e),
                }),
            )
        }
    }
}
