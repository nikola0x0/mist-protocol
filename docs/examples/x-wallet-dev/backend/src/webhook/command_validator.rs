//! Command validation module for pre-filtering tweets before queuing.
//!
//! This module provides basic regex validation to filter out invalid commands
//! before they are pushed to the processing queue, reducing unnecessary enclave calls.

use regex::Regex;
use std::sync::LazyLock;
use tracing::debug;

/// Compiled regex patterns for valid commands (compiled once, reused)
static COMMAND_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        // Update handle: @bot update [handle]
        // Note: Must check before create_account since "update" could conflict
        (
            "update_handle",
            Regex::new(r"(?i)@\w+\s+update(?:\s+handle)?").unwrap(),
        ),
        // Create account: @bot create [account] OR @bot init [account]
        (
            "create_account",
            Regex::new(r"(?i)@\w+\s+(create|init)(\s+account)?").unwrap(),
        ),
        // Link wallet: @bot link [wallet] 0x...
        (
            "link_wallet",
            Regex::new(r"(?i)@\w+\s+link\s+(?:wallet\s+)?(0x[a-fA-F0-9]{1,64})").unwrap(),
        ),
        // Transfer NFT by ID: @bot send nft 0x<nft_id> to @<receiver>
        // Note: Must check before regular transfer and NFT by name since it's more specific
        (
            "transfer_nft",
            Regex::new(r"(?i)@\w+\s+send\s+nft\s+(0x[a-fA-F0-9]{1,64})\s+to\s+(@?\w+(?:\.\w+)?)").unwrap(),
        ),
        // Transfer NFT by name: @bot send nft "<nft_name>" to @<receiver>
        // NFT name is in quotes, can contain spaces and special chars
        (
            "transfer_nft_by_name",
            Regex::new(r#"(?i)@\w+\s+send\s+nft\s+"([^"]+)"\s+to\s+(@?\w+(?:\.\w+)?)"#).unwrap(),
        ),
        // Transfer: @bot send <amount> <coin> to @<receiver> or to <name.sui>
        // Supports: @handle, name, name.sui
        (
            "transfer",
            Regex::new(r"(?i)@\w+\s+send\s+(\d+(?:\.\d+)?)\s+(\w+)\s+to\s+(@?\w+(?:\.\w+)?)").unwrap(),
        ),
    ]
});

/// Check if tweet text contains a valid command pattern.
///
/// This is a pre-filter to reduce unnecessary enclave calls.
/// Returns the matched command type name if valid, None otherwise.
pub fn validate_command(text: &str) -> Option<&'static str> {
    for (name, pattern) in COMMAND_PATTERNS.iter() {
        if pattern.is_match(text) {
            debug!("Tweet matches command pattern: {}", name);
            return Some(name);
        }
    }
    None
}

/// Check if tweet text contains any valid command pattern.
/// Used primarily in tests.
#[allow(dead_code)]
pub fn is_valid_command(text: &str) -> bool {
    validate_command(text).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Create Account Tests =====

    #[test]
    fn test_create_account_basic() {
        assert!(is_valid_command("@NautilusXWallet create"));
        assert!(is_valid_command("@NautilusXWallet init"));
    }

    #[test]
    fn test_create_account_with_account_keyword() {
        assert!(is_valid_command("@NautilusXWallet create account"));
        assert!(is_valid_command("@NautilusXWallet init account"));
    }

    #[test]
    fn test_create_account_case_insensitive() {
        assert!(is_valid_command("@NautilusXWallet CREATE"));
        assert!(is_valid_command("@NautilusXWallet INIT ACCOUNT"));
        assert!(is_valid_command("@NautilusXWallet Create Account"));
    }

    #[test]
    fn test_create_account_in_sentence() {
        assert!(is_valid_command("Hey @NautilusXWallet create account please"));
        assert!(is_valid_command("I want to @bot init my wallet"));
    }

    // ===== Link Wallet Tests =====

    #[test]
    fn test_link_wallet_basic() {
        assert!(is_valid_command(
            "@NautilusXWallet link 0x1234567890abcdef"
        ));
    }

    #[test]
    fn test_link_wallet_with_wallet_keyword() {
        assert!(is_valid_command(
            "@NautilusXWallet link wallet 0x1234567890abcdef"
        ));
    }

    #[test]
    fn test_link_wallet_full_address() {
        assert!(is_valid_command(
            "@NautilusXWallet link 0xe9209e4c3c14d931af54d10962672f743dbe08b59fabdd8cdcbebd672f11db2d"
        ));
    }

    #[test]
    fn test_link_wallet_case_insensitive() {
        assert!(is_valid_command(
            "@NautilusXWallet LINK WALLET 0xABCDEF"
        ));
    }

    // ===== Transfer Tests =====

    #[test]
    fn test_transfer_basic() {
        assert!(is_valid_command("@NautilusXWallet send 1 SUI to @user"));
        assert!(is_valid_command("@NautilusXWallet send 100 USDC to @alice"));
    }

    #[test]
    fn test_transfer_decimal_amount() {
        assert!(is_valid_command("@NautilusXWallet send 1.5 SUI to @user"));
        assert!(is_valid_command("@NautilusXWallet send 0.001 ETH to @bob"));
    }

    #[test]
    fn test_transfer_case_insensitive() {
        assert!(is_valid_command("@NautilusXWallet SEND 1 SUI TO @user"));
        assert!(is_valid_command("@NautilusXWallet Send 1 sui To @User"));
    }

    #[test]
    fn test_transfer_to_nft_name() {
        // NFT name with .sui suffix
        assert!(is_valid_command("@NautilusXWallet send 1 SUI to alice.sui"));
        assert!(is_valid_command("@NautilusXWallet send 100 USDC to bob.sui"));
        // NFT name without suffix
        assert!(is_valid_command("@NautilusXWallet send 1 SUI to alice"));
        assert!(is_valid_command("@NautilusXWallet send 0.5 WAL to myname"));
    }

    // ===== Transfer NFT Tests =====

    #[test]
    fn test_transfer_nft_basic() {
        assert!(is_valid_command(
            "@NautilusXWallet send nft 0x1234567890abcdef to @user"
        ));
    }

    #[test]
    fn test_transfer_nft_full_address() {
        assert!(is_valid_command(
            "@NautilusXWallet send nft 0xe9209e4c3c14d931af54d10962672f743dbe08b59fabdd8cdcbebd672f11db2d to @alice"
        ));
    }

    #[test]
    fn test_transfer_nft_case_insensitive() {
        assert!(is_valid_command(
            "@NautilusXWallet SEND NFT 0xABCDEF TO @user"
        ));
    }

    #[test]
    fn test_transfer_nft_to_nft_name() {
        // NFT name with .sui suffix
        assert!(is_valid_command(
            "@NautilusXWallet send nft 0x1234567890abcdef to alice.sui"
        ));
        // NFT name without suffix
        assert!(is_valid_command(
            "@NautilusXWallet send nft 0x1234567890abcdef to bob"
        ));
    }

    // ===== Transfer NFT by Name Tests =====

    #[test]
    fn test_transfer_nft_by_name_basic() {
        assert!(is_valid_command(
            r#"@NautilusXWallet send nft "Walrus Blob (445556b)" to @user"#
        ));
        assert!(is_valid_command(
            r#"@NautilusXWallet send nft "My Cool NFT" to @alice"#
        ));
    }

    #[test]
    fn test_transfer_nft_by_name_to_nft_name() {
        assert!(is_valid_command(
            r#"@NautilusXWallet send nft "Walrus Blob (445556b)" to alice.sui"#
        ));
        assert!(is_valid_command(
            r#"@NautilusXWallet send nft "My NFT" to bob"#
        ));
    }

    #[test]
    fn test_transfer_nft_by_name_case_insensitive() {
        assert!(is_valid_command(
            r#"@NautilusXWallet SEND NFT "My NFT" TO @user"#
        ));
    }

    // ===== Update Handle Tests =====

    #[test]
    fn test_update_handle_basic() {
        assert!(is_valid_command("@NautilusXWallet update handle"));
        assert!(is_valid_command("@NautilusXWallet update"));
    }

    #[test]
    fn test_update_handle_case_insensitive() {
        assert!(is_valid_command("@NautilusXWallet UPDATE HANDLE"));
        assert!(is_valid_command("@NautilusXWallet Update Handle"));
        assert!(is_valid_command("@NautilusXWallet UPDATE"));
    }

    // ===== Invalid Command Tests =====

    #[test]
    fn test_invalid_no_command() {
        assert!(!is_valid_command("@NautilusXWallet hello"));
        assert!(!is_valid_command("@NautilusXWallet what's up"));
    }

    #[test]
    fn test_invalid_incomplete_transfer() {
        assert!(!is_valid_command("@NautilusXWallet send 1 SUI"));
        assert!(!is_valid_command("@NautilusXWallet send to @user"));
    }

    #[test]
    fn test_invalid_incomplete_link() {
        assert!(!is_valid_command("@NautilusXWallet link"));
        assert!(!is_valid_command("@NautilusXWallet link wallet"));
    }

    #[test]
    fn test_invalid_no_bot_mention() {
        assert!(!is_valid_command("create account"));
        assert!(!is_valid_command("send 1 SUI to @user"));
    }

    #[test]
    fn test_invalid_random_text() {
        assert!(!is_valid_command("Hello world"));
        assert!(!is_valid_command("@user thanks for the help!"));
    }

    // ===== validate_command returns correct type =====

    #[test]
    fn test_validate_command_returns_type() {
        assert_eq!(
            validate_command("@bot create account"),
            Some("create_account")
        );
        assert_eq!(
            validate_command("@bot link 0x123"),
            Some("link_wallet")
        );
        assert_eq!(
            validate_command("@bot send 1 SUI to @user"),
            Some("transfer")
        );
        assert_eq!(
            validate_command("@bot send nft 0x123 to @user"),
            Some("transfer_nft")
        );
        assert_eq!(
            validate_command(r#"@bot send nft "My NFT" to @user"#),
            Some("transfer_nft_by_name")
        );
        assert_eq!(
            validate_command("@bot update handle"),
            Some("update_handle")
        );
        assert_eq!(
            validate_command("@bot update"),
            Some("update_handle")
        );
        assert_eq!(validate_command("@bot hello"), None);
    }
}
