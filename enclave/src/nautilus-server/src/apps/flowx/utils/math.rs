//! FlowX math utilities for CLMM calculations

/// Q64.64 fixed point number constant: 2^64
const Q64: u128 = 1u128 << 64;

/// Calculate sqrt_price from actual price
/// sqrt_price = sqrt(price) * 2^64 (Q64.64 format)
///
/// # Arguments
/// * `price` - Price: 1 token X = price token Y
/// * `decimals_x` - Decimals of token X
/// * `decimals_y` - Decimals of token Y
pub fn calculate_sqrt_price(price: f64, decimals_x: u8, decimals_y: u8) -> u128 {
    // Adjust price by decimals
    let decimal_adjustment = 10f64.powi(decimals_y as i32 - decimals_x as i32);
    let adjusted_price = price * decimal_adjustment;

    // sqrt(price) * 2^64
    let sqrt_price = adjusted_price.sqrt() * (Q64 as f64);

    sqrt_price as u128
}

/// Calculate price from sqrt_price
pub fn sqrt_price_to_price(sqrt_price: u128, decimals_x: u8, decimals_y: u8) -> f64 {
    let sqrt_price_f64 = sqrt_price as f64 / Q64 as f64;
    let price = sqrt_price_f64 * sqrt_price_f64;

    let decimal_adjustment = 10f64.powi(decimals_x as i32 - decimals_y as i32);
    price * decimal_adjustment
}

/// Calculate tick from sqrt_price
/// tick = log_{1.0001}(sqrt_price^2) = 2 * log_{1.0001}(sqrt_price)
pub fn sqrt_price_to_tick(sqrt_price: u128) -> i32 {
    let sqrt_price_f64 = sqrt_price as f64 / Q64 as f64;
    let price = sqrt_price_f64 * sqrt_price_f64;

    // tick = log(price) / log(1.0001)
    let tick = price.ln() / 1.0001f64.ln();
    tick.floor() as i32
}

/// Calculate sqrt_price from tick
/// sqrt_price = sqrt(1.0001^tick) * 2^64
pub fn tick_to_sqrt_price(tick: i32) -> u128 {
    let price = 1.0001f64.powi(tick);
    let sqrt_price = price.sqrt() * (Q64 as f64);
    sqrt_price as u128
}

/// Tick spacing by fee tier
pub fn fee_to_tick_spacing(fee_rate: u64) -> i32 {
    match fee_rate {
        100 => 1,      // 0.01%
        500 => 10,     // 0.05%
        3000 => 60,    // 0.3%
        10000 => 200,  // 1%
        _ => 60,       // Default
    }
}

/// Calculate slippage price limit
pub fn calculate_price_limit(current_sqrt_price: u128, slippage_bps: u64, is_x_to_y: bool) -> u128 {
    if is_x_to_y {
        // Price decreases when swapping X -> Y, set lower limit
        let factor = 10000 - slippage_bps;
        (current_sqrt_price * factor as u128) / 10000
    } else {
        // Price increases when swapping Y -> X, set upper limit
        let factor = 10000 + slippage_bps;
        (current_sqrt_price * factor as u128) / 10000
    }
}

/// Min/Max sqrt price constants (from FlowX)
pub const MIN_SQRT_PRICE: u128 = 4295048016;
pub const MAX_SQRT_PRICE: u128 = 79226673515401279992447579055;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_price_calculation() {
        // 1 SUI = 100 TOKEN (both 9 decimals)
        let sqrt_price = calculate_sqrt_price(100.0, 9, 9);
        let price_back = sqrt_price_to_price(sqrt_price, 9, 9);

        assert!((price_back - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_tick_conversion() {
        let sqrt_price = calculate_sqrt_price(1.0, 9, 9);
        let tick = sqrt_price_to_tick(sqrt_price);

        // Price = 1 should give tick ≈ 0
        assert!(tick.abs() < 10);
    }
}
