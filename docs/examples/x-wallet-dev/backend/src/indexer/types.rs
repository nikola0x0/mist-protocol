// Re-export types from sui_client for convenience
pub use crate::clients::sui_client::{EventPage, SuiEvent};

use serde::{Deserialize, Serialize};

/// XWallet-specific event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCreatedEvent {
    pub xid: String,
    pub handle: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletLinkedEvent {
    pub xid: String,
    pub owner_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCompletedEvent {
    pub from_xid: String,
    pub to_xid: String,
    pub tweet_id: String,
    pub coin_type: String,
    pub amount: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinDepositedEvent {
    pub xid: String,
    pub coin_type: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinWithdrawnEvent {
    pub xid: String,
    pub coin_type: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleUpdatedEvent {
    pub xid: String,
    pub old_handle: String,
    pub new_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftDepositedEvent {
    pub xid: String,
    pub nft_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftWithdrawnEvent {
    pub xid: String,
    pub nft_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftTransferredEvent {
    pub from_xid: String,
    pub to_xid: String,
    pub nft_id: String,
    pub timestamp: String,
}

/// Parse event type from full event type string
pub fn parse_event_type(full_type: &str) -> Option<&str> {
    // Example: 0x...::events::AccountCreated -> AccountCreated
    full_type.split("::").last()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====== parse_event_type tests ======

    #[test]
    fn test_parse_event_type_account_created() {
        let full_type = "0x123abc::events::AccountCreated";
        assert_eq!(parse_event_type(full_type), Some("AccountCreated"));
    }

    #[test]
    fn test_parse_event_type_coin_transferred() {
        let full_type = "0x456def::xwallet::CoinTransferred";
        assert_eq!(parse_event_type(full_type), Some("CoinTransferred"));
    }

    #[test]
    fn test_parse_event_type_wallet_linked() {
        let full_type = "0x789::module::WalletLinked";
        assert_eq!(parse_event_type(full_type), Some("WalletLinked"));
    }

    #[test]
    fn test_parse_event_type_simple() {
        let full_type = "SimpleEvent";
        assert_eq!(parse_event_type(full_type), Some("SimpleEvent"));
    }

    #[test]
    fn test_parse_event_type_empty() {
        let full_type = "";
        assert_eq!(parse_event_type(full_type), Some(""));
    }

    #[test]
    fn test_parse_event_type_long_path() {
        let full_type = "0x1234567890abcdef::very::long::path::to::EventName";
        assert_eq!(parse_event_type(full_type), Some("EventName"));
    }

    // ====== Event struct serialization tests ======

    #[test]
    fn test_account_created_event_serialization() {
        let event = AccountCreatedEvent {
            xid: "123456".to_string(),
            handle: "test_user".to_string(),
            account_id: "0xabc123".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("123456"));
        assert!(json.contains("test_user"));
        assert!(json.contains("0xabc123"));

        // Deserialize back
        let parsed: AccountCreatedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.xid, "123456");
        assert_eq!(parsed.handle, "test_user");
        assert_eq!(parsed.account_id, "0xabc123");
    }

    #[test]
    fn test_wallet_linked_event_serialization() {
        let event = WalletLinkedEvent {
            xid: "789".to_string(),
            owner_address: "0xowner123".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: WalletLinkedEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.xid, "789");
        assert_eq!(parsed.owner_address, "0xowner123");
    }

    #[test]
    fn test_transfer_completed_event_serialization() {
        let event = TransferCompletedEvent {
            from_xid: "sender123".to_string(),
            to_xid: "receiver456".to_string(),
            tweet_id: "tweet789".to_string(),
            coin_type: "0x2::sui::SUI".to_string(),
            amount: "1000000000".to_string(),
            timestamp: "1234567890".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: TransferCompletedEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.from_xid, "sender123");
        assert_eq!(parsed.to_xid, "receiver456");
        assert_eq!(parsed.amount, "1000000000");
    }

    #[test]
    fn test_coin_deposited_event_serialization() {
        let event = CoinDepositedEvent {
            xid: "user123".to_string(),
            coin_type: "USDC".to_string(),
            amount: "500000".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: CoinDepositedEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.xid, "user123");
        assert_eq!(parsed.coin_type, "USDC");
    }

    #[test]
    fn test_nft_transferred_event_serialization() {
        let event = NftTransferredEvent {
            from_xid: "from_user".to_string(),
            to_xid: "to_user".to_string(),
            nft_id: "0xnft123".to_string(),
            timestamp: "9876543210".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: NftTransferredEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.from_xid, "from_user");
        assert_eq!(parsed.nft_id, "0xnft123");
    }
}
