// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration and utility functions for XWallet enclave
//!
//! Contains coin type utilities, hex encoding/decoding, and other helpers.

/// Hex encoding/decoding for addresses
pub mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let bytes = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Hex decode error: {}", e))?;
        Ok(bytes)
    }

    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Get decimals for a coin type
pub fn get_coin_decimals(coin_type: &str) -> u32 {
    match coin_type.to_uppercase().as_str() {
        "SUI" => 9,
        "WAL" => 9,
        "USDC" => 6,
        _ => 9, // Default to 9 decimals if unknown
    }
}

/// Expand shorthand coin types to full type paths
/// Uses coin type addresses from AppState (configurable via environment variables)
pub fn expand_coin_type(coin_type: &str, usdc_type: &str, wal_type: &str) -> String {
    match coin_type.to_uppercase().as_str() {
        "SUI" => "0x2::sui::SUI".to_string(),
        "USDC" => usdc_type.to_string(),
        "WAL" => wal_type.to_string(),
        _ => {
            // If it's already a full type path, use as-is
            if coin_type.contains("::") {
                coin_type.to_string()
            } else {
                coin_type.to_string()
            }
        }
    }
}

/// Convert coin type to canonical format expected by Move's `type_name::get<T>()`
/// Example: "0x2::sui::SUI" -> "0000000000000000000000000000000000000000000000000000000000000002::sui::SUI"
pub fn to_canonical_coin_type(coin_type: &str, usdc_type: &str, wal_type: &str) -> String {
    let expanded = expand_coin_type(coin_type, usdc_type, wal_type);

    if let Some(rest) = expanded.strip_prefix("0x") {
        if let Some(idx) = rest.find("::") {
            let addr = &rest[..idx];
            let module_and_type = &rest[idx..];
            let canonical_addr = format!("{:0>64}", addr);
            return format!("{}{}", canonical_addr, module_and_type);
        }
    }

    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode_decode() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = hex::encode(&bytes);
        assert_eq!(encoded, "deadbeef");

        let decoded = hex::decode(&encoded).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn test_get_coin_decimals() {
        assert_eq!(get_coin_decimals("SUI"), 9);
        assert_eq!(get_coin_decimals("sui"), 9);
        assert_eq!(get_coin_decimals("USDC"), 6);
        assert_eq!(get_coin_decimals("WAL"), 9);
        assert_eq!(get_coin_decimals("unknown"), 9);
    }

    #[test]
    fn test_expand_coin_type() {
        let usdc = "0xtest::usdc::USDC";
        let wal = "0xtest::wal::WAL";

        assert_eq!(expand_coin_type("SUI", usdc, wal), "0x2::sui::SUI");
        assert_eq!(expand_coin_type("USDC", usdc, wal), usdc);
        assert_eq!(expand_coin_type("WAL", usdc, wal), wal);
        assert_eq!(expand_coin_type("0xcustom::token::TOKEN", usdc, wal), "0xcustom::token::TOKEN");
    }

    #[test]
    fn test_to_canonical_coin_type() {
        let usdc = "0xtest::usdc::USDC";
        let wal = "0xtest::wal::WAL";

        let canonical = to_canonical_coin_type("SUI", usdc, wal);
        assert!(canonical.starts_with("0000000000000000000000000000000000000000000000000000000000000002"));
        assert!(canonical.ends_with("::sui::SUI"));
    }
}
