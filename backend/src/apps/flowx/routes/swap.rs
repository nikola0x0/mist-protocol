use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::apps::flowx::services::FlowXService;

/// Request để swap token
#[derive(Debug, Deserialize)]
pub struct SwapRequest {
    /// Số lượng token đầu vào (đơn vị nhỏ nhất)
    pub amount_in: u64,
    /// Số lượng tối thiểu nhận được (slippage protection)
    pub min_amount_out: u64,
    /// true = SUI -> YOUR_TOKEN, false = YOUR_TOKEN -> SUI
    pub is_sui_to_token: bool,
    /// Slippage tolerance (basis points, ví dụ: 50 = 0.5%)
    #[serde(default = "default_slippage")]
    pub slippage_bps: u64,
    /// Fee tier của pool
    #[serde(default = "default_fee")]
    pub fee_rate: u64,
}

fn default_slippage() -> u64 {
    50 // 0.5%
}

fn default_fee() -> u64 {
    3000 // 0.3%
}

/// Response sau khi swap
#[derive(Debug, Serialize)]
pub struct SwapResponse {
    pub success: bool,
    pub tx_digest: Option<String>,
    pub amount_in: u64,
    pub amount_out: Option<u64>,
    pub message: String,
}

/// Request để quote (ước tính) swap
#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub amount_in: u64,
    pub is_sui_to_token: bool,
    #[serde(default = "default_fee")]
    pub fee_rate: u64,
}

/// Response quote
#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub amount_in: u64,
    pub estimated_amount_out: u64,
    pub price_impact: f64,
    pub fee_amount: u64,
}

/// Thực hiện swap
pub async fn swap(
    State(service): State<Arc<FlowXService>>,
    Json(req): Json<SwapRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Swap request: {} {} -> {}, min_out={}",
        req.amount_in,
        if req.is_sui_to_token { "SUI" } else { "TOKEN" },
        if req.is_sui_to_token { "TOKEN" } else { "SUI" },
        req.min_amount_out
    );
    
    // Validate
    if req.amount_in == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(SwapResponse {
                success: false,
                tx_digest: None,
                amount_in: req.amount_in,
                amount_out: None,
                message: "amount_in must be greater than 0".to_string(),
            }),
        );
    }
    
    if req.slippage_bps > 5000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(SwapResponse {
                success: false,
                tx_digest: None,
                amount_in: req.amount_in,
                amount_out: None,
                message: "slippage_bps too high (max 50%)".to_string(),
            }),
        );
    }
    
    match service
        .swap_exact_input(
            req.amount_in,
            req.min_amount_out,
            req.is_sui_to_token,
            req.slippage_bps,
            req.fee_rate,
        )
        .await
    {
        Ok(tx_digest) => (
            StatusCode::OK,
            Json(SwapResponse {
                success: true,
                tx_digest: Some(tx_digest),
                amount_in: req.amount_in,
                amount_out: None, // TODO: Parse from transaction effects
                message: "Swap executed successfully".to_string(),
            }),
        ),
        Err(e) => {
            tracing::error!("Swap failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SwapResponse {
                    success: false,
                    tx_digest: None,
                    amount_in: req.amount_in,
                    amount_out: None,
                    message: format!("Swap failed: {}", e),
                }),
            )
        }
    }
}

/// Ước tính kết quả swap (không thực hiện transaction)
pub async fn quote(
    State(_service): State<Arc<FlowXService>>,
    Json(req): Json<QuoteRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Quote request: {} {} -> {}",
        req.amount_in,
        if req.is_sui_to_token { "SUI" } else { "TOKEN" },
        if req.is_sui_to_token { "TOKEN" } else { "SUI" }
    );
    
    // TODO: Implement proper quote calculation
    // Cần đọc state của pool để tính chính xác
    // Đây là placeholder
    
    let fee_amount = req.amount_in * req.fee_rate / 1_000_000;
    let amount_after_fee = req.amount_in - fee_amount;
    
    // Simplified estimation (thực tế cần tính theo liquidity và price)
    let estimated_amount_out = amount_after_fee; // 1:1 placeholder
    let price_impact = 0.1; // 0.1% placeholder
    
    (
        StatusCode::OK,
        Json(QuoteResponse {
            amount_in: req.amount_in,
            estimated_amount_out,
            price_impact,
            fee_amount,
        }),
    )
}

/// Lấy thông tin giá hiện tại
#[derive(Debug, Serialize)]
pub struct PriceResponse {
    pub price: f64,
    pub sqrt_price: String,
    pub tick: i32,
}

pub async fn get_price(
    State(_service): State<Arc<FlowXService>>,
) -> impl IntoResponse {
    // TODO: Implement actual price fetching from pool
    // Đây là placeholder
    
    (
        StatusCode::OK,
        Json(PriceResponse {
            price: 100.0, // 1 SUI = 100 TOKEN
            sqrt_price: "184467440737095516160".to_string(),
            tick: 46054,
        }),
    )
}
