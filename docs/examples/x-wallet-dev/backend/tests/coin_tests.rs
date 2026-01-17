//! Unit tests for coin formatting utilities
//!
//! Tests the `constants::coin` module functions for:
//! - Getting coin info (symbol, decimals)
//! - Formatting amounts
//! - Case-insensitive coin type detection

#[cfg(test)]
mod coin_info_tests {
    use xwallet_backend::constants::coin::{get_coin_info, SUI, USDC, WAL};

    #[test]
    fn test_get_coin_info_sui_simple() {
        let info = get_coin_info("SUI");
        assert_eq!(info.symbol, "SUI");
        assert_eq!(info.decimals, 9);
    }

    #[test]
    fn test_get_coin_info_sui_full_path() {
        let info = get_coin_info("0x2::sui::SUI");
        assert_eq!(info.symbol, "SUI");
        assert_eq!(info.decimals, 9);
    }

    #[test]
    fn test_get_coin_info_usdc_simple() {
        let info = get_coin_info("USDC");
        assert_eq!(info.symbol, "USDC");
        assert_eq!(info.decimals, 6);
    }

    #[test]
    fn test_get_coin_info_usdc_full_path() {
        let info = get_coin_info("0x5d4b302506645c37ff133b98c4b50a5ae14841659738d6d733d59d0d217a93bf::usdc::USDC");
        assert_eq!(info.symbol, "USDC");
        assert_eq!(info.decimals, 6);
    }

    #[test]
    fn test_get_coin_info_wal_simple() {
        let info = get_coin_info("WAL");
        assert_eq!(info.symbol, "WAL");
        assert_eq!(info.decimals, 9);
    }

    #[test]
    fn test_get_coin_info_wal_full_path() {
        let info = get_coin_info("0x123::wal::WAL");
        assert_eq!(info.symbol, "WAL");
        assert_eq!(info.decimals, 9);
    }

    #[test]
    fn test_get_coin_info_case_insensitive_sui() {
        // All variations should return SUI
        let variations = vec!["sui", "SUI", "Sui", "0x2::sui::SUI", "0x2::SUI::SUI"];
        for coin_type in variations {
            let info = get_coin_info(coin_type);
            assert_eq!(info.decimals, 9, "Failed for: {}", coin_type);
        }
    }

    #[test]
    fn test_get_coin_info_case_insensitive_usdc() {
        // All variations should return USDC with 6 decimals
        let variations = vec!["usdc", "USDC", "Usdc", "0x123::usdc::USDC", "0x123::USDC::USDC"];
        for coin_type in variations {
            let info = get_coin_info(coin_type);
            assert_eq!(info.decimals, 6, "Failed for: {}", coin_type);
        }
    }

    #[test]
    fn test_get_coin_info_unknown_coin() {
        let info = get_coin_info("0x123::unknown::UNKNOWN");
        assert_eq!(info.symbol, "UNKNOWN");
        assert_eq!(info.decimals, 9); // Default decimals
    }

    #[test]
    fn test_get_coin_info_custom_token() {
        let info = get_coin_info("0xabc::mytoken::MYTOKEN");
        assert_eq!(info.symbol, "MYTOKEN");
        assert_eq!(info.decimals, 9);
    }

    #[test]
    fn test_coin_constants() {
        assert_eq!(SUI.symbol, "SUI");
        assert_eq!(SUI.decimals, 9);

        assert_eq!(USDC.symbol, "USDC");
        assert_eq!(USDC.decimals, 6);

        assert_eq!(WAL.symbol, "WAL");
        assert_eq!(WAL.decimals, 9);
    }
}

#[cfg(test)]
mod format_amount_tests {
    use xwallet_backend::constants::coin::{format_amount, format_amount_with_symbol};

    #[test]
    fn test_format_amount_one_sui() {
        // 1 SUI = 1_000_000_000 mist
        let result = format_amount(1_000_000_000, 9);
        assert_eq!(result, "1");
    }

    #[test]
    fn test_format_amount_one_usdc() {
        // 1 USDC = 1_000_000 micro units
        let result = format_amount(1_000_000, 6);
        assert_eq!(result, "1");
    }

    #[test]
    fn test_format_amount_fractional_sui() {
        // 0.5 SUI
        let result = format_amount(500_000_000, 9);
        assert_eq!(result, "0.5");
    }

    #[test]
    fn test_format_amount_fractional_usdc() {
        // 0.5 USDC
        let result = format_amount(500_000, 6);
        assert_eq!(result, "0.5");
    }

    #[test]
    fn test_format_amount_trim_trailing_zeros() {
        // 1.5 SUI (not 1.500000000)
        let result = format_amount(1_500_000_000, 9);
        assert_eq!(result, "1.5");
    }

    #[test]
    fn test_format_amount_trim_decimal_point() {
        // Whole number should not have decimal point
        let result = format_amount(2_000_000_000, 9);
        assert_eq!(result, "2");
    }

    #[test]
    fn test_format_amount_small_value_usdc() {
        // 0.0006 USDC = 600 micro units
        let result = format_amount(600, 6);
        assert_eq!(result, "0.0006");
    }

    #[test]
    fn test_format_amount_very_small_value() {
        // 1 micro unit of USDC = 0.000001
        let result = format_amount(1, 6);
        assert_eq!(result, "0.000001");
    }

    #[test]
    fn test_format_amount_zero() {
        let result = format_amount(0, 9);
        assert_eq!(result, "0");
    }

    #[test]
    fn test_format_amount_large_value() {
        // 1000 SUI
        let result = format_amount(1_000_000_000_000, 9);
        assert_eq!(result, "1000");
    }

    #[test]
    fn test_format_amount_precise_value() {
        // 1.123456789 SUI
        let result = format_amount(1_123_456_789, 9);
        assert_eq!(result, "1.123456789");
    }

    #[test]
    fn test_format_amount_with_symbol_sui() {
        let result = format_amount_with_symbol(1_500_000_000, "0x2::sui::SUI");
        assert_eq!(result, "1.5 SUI");
    }

    #[test]
    fn test_format_amount_with_symbol_usdc() {
        let result = format_amount_with_symbol(1_500_000, "0x123::usdc::USDC");
        assert_eq!(result, "1.5 USDC");
    }

    #[test]
    fn test_format_amount_with_symbol_small_usdc() {
        // This was the bug - 600 micro USDC should be 0.0006, not 0.0000006
        let result = format_amount_with_symbol(600, "USDC");
        assert_eq!(result, "0.0006 USDC");
    }

    #[test]
    fn test_format_amount_with_symbol_unknown() {
        let result = format_amount_with_symbol(1_000_000_000, "0x123::custom::CUSTOM");
        assert_eq!(result, "1 CUSTOM");
    }
}
