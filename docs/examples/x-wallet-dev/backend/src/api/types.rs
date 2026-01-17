//! Shared types for API responses

use serde::{Deserialize, Serialize};

use crate::db::models::XWalletAccount;

/// Basic account response
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub x_user_id: String,
    pub x_handle: String,
    pub sui_object_id: String,
    pub owner_address: Option<String>,
    pub avatar_url: Option<String>,
}

impl From<XWalletAccount> for AccountResponse {
    fn from(account: XWalletAccount) -> Self {
        Self {
            x_user_id: account.x_user_id,
            x_handle: account.x_handle,
            sui_object_id: account.sui_object_id,
            owner_address: account.owner_address,
            avatar_url: account.avatar_url,
        }
    }
}

/// Balance response for a single token
#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub coin_type: String,
    pub balance: String,
}

/// Account detail response with balances
#[derive(Debug, Serialize)]
pub struct AccountDetailResponse {
    pub account: AccountResponse,
    pub balances: Vec<BalanceResponse>,
}

/// Token balance with full info
#[derive(Debug, Serialize)]
pub struct TokenBalance {
    pub symbol: String,
    pub coin_type: String,
    pub balance_raw: i64,
    pub balance_formatted: String,
    pub decimals: u8,
}

/// Generic pagination query params
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== AccountResponse tests ======

    #[test]
    fn test_account_response_serialize() {
        let response = AccountResponse {
            x_user_id: "123456".to_string(),
            x_handle: "testuser".to_string(),
            sui_object_id: "0xabc123".to_string(),
            owner_address: Some("0xowner456".to_string()),
            avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"x_user_id\":\"123456\""));
        assert!(json.contains("\"x_handle\":\"testuser\""));
        assert!(json.contains("\"sui_object_id\":\"0xabc123\""));
        assert!(json.contains("\"owner_address\":\"0xowner456\""));
    }

    #[test]
    fn test_account_response_deserialize() {
        let json = r#"{
            "x_user_id": "123456",
            "x_handle": "testuser",
            "sui_object_id": "0xabc123",
            "owner_address": null
        }"#;

        let response: AccountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.x_user_id, "123456");
        assert_eq!(response.x_handle, "testuser");
        assert_eq!(response.sui_object_id, "0xabc123");
        assert!(response.owner_address.is_none());
    }

    #[test]
    fn test_account_response_with_owner() {
        let json = r#"{
            "x_user_id": "789",
            "x_handle": "user2",
            "sui_object_id": "0xdef",
            "owner_address": "0xowner"
        }"#;

        let response: AccountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.owner_address, Some("0xowner".to_string()));
    }

    // ====== BalanceResponse tests ======

    #[test]
    fn test_balance_response_serialize() {
        let balance = BalanceResponse {
            coin_type: "0x2::sui::SUI".to_string(),
            balance: "1.5".to_string(),
        };

        let json = serde_json::to_string(&balance).unwrap();
        assert!(json.contains("\"coin_type\":\"0x2::sui::SUI\""));
        assert!(json.contains("\"balance\":\"1.5\""));
    }

    // ====== TokenBalance tests ======

    #[test]
    fn test_token_balance_serialize() {
        let token = TokenBalance {
            symbol: "SUI".to_string(),
            coin_type: "0x2::sui::SUI".to_string(),
            balance_raw: 1_500_000_000,
            balance_formatted: "1.5".to_string(),
            decimals: 9,
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("\"symbol\":\"SUI\""));
        assert!(json.contains("\"balance_raw\":1500000000"));
        assert!(json.contains("\"balance_formatted\":\"1.5\""));
        assert!(json.contains("\"decimals\":9"));
    }

    #[test]
    fn test_token_balance_debug() {
        let token = TokenBalance {
            symbol: "USDC".to_string(),
            coin_type: "usdc".to_string(),
            balance_raw: 1_000_000,
            balance_formatted: "1".to_string(),
            decimals: 6,
        };

        let debug_str = format!("{:?}", token);
        assert!(debug_str.contains("USDC"));
        assert!(debug_str.contains("1000000"));
    }

    // ====== PaginationQuery tests ======

    #[test]
    fn test_pagination_query_deserialize_full() {
        let json = r#"{"limit": 10, "page": 2}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.page, Some(2));
    }

    #[test]
    fn test_pagination_query_deserialize_partial() {
        let json = r#"{"limit": 5}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(5));
        assert!(query.page.is_none());
    }

    #[test]
    fn test_pagination_query_deserialize_empty() {
        let json = r#"{}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert!(query.limit.is_none());
        assert!(query.page.is_none());
    }

    // ====== AccountDetailResponse tests ======

    #[test]
    fn test_account_detail_response_serialize() {
        let detail = AccountDetailResponse {
            account: AccountResponse {
                x_user_id: "123".to_string(),
                x_handle: "user".to_string(),
                sui_object_id: "0xobj".to_string(),
                owner_address: None,
                avatar_url: None,
            },
            balances: vec![
                BalanceResponse {
                    coin_type: "SUI".to_string(),
                    balance: "10".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("\"account\":{"));
        assert!(json.contains("\"balances\":["));
        assert!(json.contains("\"coin_type\":\"SUI\""));
    }
}
