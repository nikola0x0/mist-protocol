// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! XWallet Enclave Server Module
//!
//! Processes Twitter-based transfer commands with signatures for Sui blockchain.
//!
//! ## Module Structure
//!
//! - `types`: Request/response structs and payload definitions
//! - `config`: Coin type utilities and hex encoding
//! - `twitter`: Twitter API integration
//! - `signatures`: Sui wallet signature verification
//! - `commands`: Tweet command parsing
//! - `handlers`: HTTP endpoint handlers
//! - `sui_rpc`: Sui RPC client for NFT name lookup

// Submodules
mod commands;
pub mod config;
mod handlers;
mod signatures;
mod sui_rpc;
mod twitter;
mod types;

// Re-export types
pub use types::{
    // Payloads (for Move contract integration)
    InitAccountPayload,
    LinkWalletPayload,
    TransferNftPayload,
    TransferPayload,
    UpdateHandlePayload,
    // Request types
    InitAccountRequest,
    ProcessTweetRequest,
    SecureLinkWalletRequest,
    UpdateHandleRequest,
    // Response types
    CommandType,
    CreateAccountData,
    LinkWalletData,
    ProcessTweetData,
    ProcessTweetResponse,
    TransferData,
    TransferNftData,
    TweetCommon,
    TweetData,
    TwitterUserInfo,
    UpdateHandleData,
};

// Re-export handlers (public endpoints)
pub use handlers::{
    process_init_account,
    process_secure_link_wallet,
    process_tweet,
    process_update_handle,
};

// Re-export command parsing (for tests)
pub use commands::{parse_tweet_command_type, ParsedCommand};

// Re-export signature verification (for tests)
pub use signatures::{
    build_personal_message_intent,
    verify_address_from_pubkey,
    verify_ed25519_signature,
    verify_secp256k1_signature,
    verify_secp256r1_signature,
    verify_sui_wallet_signature,
    verify_sui_wallet_signature_async,
    verify_zklogin_address_only,
    SIG_FLAG_ED25519,
    SIG_FLAG_SECP256K1,
    SIG_FLAG_SECP256R1,
    SIG_FLAG_ZKLOGIN,
};

// Re-export config utilities (for tests)
pub use config::{expand_coin_type, get_coin_decimals, hex, to_canonical_coin_type};

// Re-export Sui RPC client
pub use sui_rpc::{NftLookupResult, NftMatch, SuiRpcClient, SuiRpcError};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{IntentMessage, IntentScope};
    use fastcrypto::encoding::{Encoding, Hex};
    use regex::Regex;

    // Test constants for coin types (mainnet values for testing)
    const TEST_USDC_TYPE: &str =
        "0xdba34672e30cb065b1f93e3ab55318768fd6fef66c15942c9f7cb846e2f900e7::usdc::USDC";
    const TEST_WAL_TYPE: &str =
        "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL";

    #[test]
    fn test_transfer_payload_serde() {
        let payload = TransferPayload {
            from_xid: b"123456789".to_vec(),
            to_xid: b"987654321".to_vec(),
            amount: 5_000_000_000,
            coin_type: to_canonical_coin_type("SUI", TEST_USDC_TYPE, TEST_WAL_TYPE).into_bytes(),
            tweet_id: b"1234567890123456789".to_vec(),
        };

        let timestamp = 1744038900000u64;
        let intent_msg = IntentMessage::new(payload, timestamp, IntentScope::ProcessData);

        let signing_payload = bcs::to_bytes(&intent_msg).expect("should not fail");
        println!("BCS hex: {}", Hex::encode(&signing_payload));

        let _deserialized: IntentMessage<TransferPayload> =
            bcs::from_bytes(&signing_payload).expect("should deserialize");
    }

    #[test]
    fn test_transfer_regex() {
        let regex = Regex::new(r"@\w+\s+send\s+(\d+(?:\.\d+)?)\s+(\w+)\s+to\s+@(\w+)").unwrap();

        let tweet1 = "@NautilusWallet send 5 SUI to @alice";
        let captures1 = regex.captures(tweet1).unwrap();
        assert_eq!(captures1.get(1).unwrap().as_str(), "5");
        assert_eq!(captures1.get(2).unwrap().as_str(), "SUI");
        assert_eq!(captures1.get(3).unwrap().as_str(), "alice");
    }

    #[test]
    fn test_parse_tweet_command_type_create_account() {
        let test_cases = vec![
            "@xwallet create account",
            "@xwallet create",
            "@xwallet init account",
            "@xwallet init",
        ];

        for tweet in test_cases {
            let result = parse_tweet_command_type(tweet, "123456789");
            assert!(result.is_ok(), "Failed for tweet: {}", tweet);
            match result.unwrap() {
                ParsedCommand::CreateAccount => {}
                other => panic!("Expected CreateAccount, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_tweet_command_type_transfer_nft_by_name() {
        // Test NFT transfer by name (quoted)
        let quoted_test_cases = vec![
            (r#"@xwallet send nft "Popkins #6408" to @alice"#, "Popkins #6408", "alice"),
            (r#"@xwallet send nft "My Cool NFT" to @bob"#, "My Cool NFT", "bob"),
        ];

        for (tweet, expected_nft_name, expected_receiver) in quoted_test_cases {
            let result = parse_tweet_command_type(tweet, "123456789");
            assert!(result.is_ok(), "Failed for tweet: {}", tweet);
            match result.unwrap() {
                ParsedCommand::TransferNftByName { nft_name, receiver_username } => {
                    assert_eq!(nft_name, expected_nft_name);
                    assert_eq!(receiver_username, expected_receiver);
                }
                other => panic!("Expected TransferNftByName, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_tweet_nft_id_takes_priority_over_name() {
        // Object ID should be parsed as TransferNft, not TransferNftByName
        let tweet = "@xwallet send nft 0x1234 to @alice";
        let result = parse_tweet_command_type(tweet, "123456789");
        assert!(result.is_ok());
        match result.unwrap() {
            ParsedCommand::TransferNft { nft_id, receiver_username } => {
                assert_eq!(nft_id, "0x1234");
                assert_eq!(receiver_username, "alice");
            }
            other => panic!("Expected TransferNft (by ID), got {:?}", other),
        }
    }

    #[test]
    fn test_command_type_serialization() {
        assert_eq!(
            serde_json::to_string(&CommandType::CreateAccount).unwrap(),
            r#""create_account""#
        );
        assert_eq!(
            serde_json::to_string(&CommandType::Transfer).unwrap(),
            r#""transfer""#
        );
        assert_eq!(
            serde_json::to_string(&CommandType::TransferNft).unwrap(),
            r#""transfer_nft""#
        );
    }

    #[test]
    fn test_build_personal_message_intent() {
        let message = "Hello, World!";
        let intent = build_personal_message_intent(message).unwrap();
        assert_eq!(&intent[0..3], &[3, 0, 0]);
        assert!(intent.len() > 3);
    }

    #[test]
    fn test_signature_scheme_detection() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let ed25519_sig = vec![0x00; 97];
        let ed25519_base64 = STANDARD.encode(&ed25519_sig);
        let result = verify_sui_wallet_signature("0x1234", "test", &ed25519_base64);
        assert!(result.is_err());
        assert!(!result.unwrap_err().to_string().contains("Unsupported signature scheme"));

        let zklogin_sig = vec![0x05; 100];
        let zklogin_base64 = STANDARD.encode(&zklogin_sig);
        let result = verify_sui_wallet_signature("0x1234", "test", &zklogin_base64);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ZkLogin"));
    }
}
