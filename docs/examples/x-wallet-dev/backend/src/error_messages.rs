//! User-friendly error messages for xWallet
//!
//! Maps Move abort codes and other errors to human-readable messages.
//! Only specific errors are shown to users via Twitter replies.

use regex::Regex;
use std::sync::LazyLock;

/// Regex to extract abort code from Move error messages
/// Example: "MoveAbort(MoveLocation { ... }, 5)" -> extracts "5"
/// The abort code is always the last number before the closing parenthesis
static ABORT_CODE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"MoveAbort\(.*,\s*(\d+)\)").unwrap()
});

/// Move contract error codes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveErrorCode {
    XidAlreadyExists = 0,
    InsufficientBalance = 5,
    NftNotFound = 6,
}

impl MoveErrorCode {
    /// Parse error code from number (only user-reportable errors)
    pub fn from_code(code: u64) -> Option<Self> {
        match code {
            0 => Some(Self::XidAlreadyExists),
            5 => Some(Self::InsufficientBalance),
            6 => Some(Self::NftNotFound),
            _ => None,
        }
    }

    /// Get user-friendly message for this error
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::XidAlreadyExists => "You already have an XWallet account! Use your existing account to send and receive crypto.",
            Self::InsufficientBalance => "Insufficient balance. Please check your account balance and try again.",
            Self::NftNotFound => "NFT not found in your account. Please verify the NFT ID.",
        }
    }
}

/// Extract abort code from error message string
pub fn extract_abort_code(error_msg: &str) -> Option<u64> {
    ABORT_CODE_REGEX
        .captures(error_msg)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Check if error should be reported to user via Twitter reply
///
/// Only returns Some(message) for specific errors that users should know about:
/// - EXidAlreadyExists (code 0): Account already exists
/// - EInsufficientBalance (code 5): Not enough funds
/// - ENftNotFound (code 6): NFT not in account
///
/// Returns None for all other errors (internal errors, signature issues, etc.)
pub fn get_user_reportable_message(error_msg: &str) -> Option<String> {
    // Try to extract Move abort code
    if let Some(code) = extract_abort_code(error_msg) {
        if let Some(error_code) = MoveErrorCode::from_code(code) {
            return Some(error_code.user_message().to_string());
        }
    }

    // Check for common error patterns that map to reportable errors
    let error_lower = error_msg.to_lowercase();

    // Insufficient balance pattern
    if error_lower.contains("insufficient") && error_lower.contains("balance") {
        return Some(MoveErrorCode::InsufficientBalance.user_message().to_string());
    }

    // NFT not found pattern
    if error_lower.contains("nft") && error_lower.contains("not found") {
        return Some(MoveErrorCode::NftNotFound.user_message().to_string());
    }

    // All other errors are not reported to users
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== extract_abort_code tests =====

    #[test]
    fn test_extract_abort_code_basic() {
        let msg = r#"MoveAbort(MoveLocation { module: ModuleId { address: 0x123 }, function: 0 }, 5)"#;
        assert_eq!(extract_abort_code(msg), Some(5));
    }

    #[test]
    fn test_extract_abort_code_insufficient_balance() {
        let msg = r#"Transaction failed: MoveAbort(MoveLocation { module: ModuleId { address: 0xabc, name: Identifier("xwallet") }, function: 2, instruction: 10, function_name: Some("transfer_coin") }, 5) in command 0"#;
        assert_eq!(extract_abort_code(msg), Some(5));
    }

    #[test]
    fn test_extract_abort_code_no_match() {
        let msg = "Some random error message";
        assert_eq!(extract_abort_code(msg), None);
    }

    // ===== MoveErrorCode tests =====

    #[test]
    fn test_move_error_code_from_code_reportable() {
        // Only 3 error codes are reportable
        assert_eq!(MoveErrorCode::from_code(0), Some(MoveErrorCode::XidAlreadyExists));
        assert_eq!(MoveErrorCode::from_code(5), Some(MoveErrorCode::InsufficientBalance));
        assert_eq!(MoveErrorCode::from_code(6), Some(MoveErrorCode::NftNotFound));
    }

    #[test]
    fn test_move_error_code_from_code_not_reportable() {
        // Other error codes return None (not reported to user)
        assert_eq!(MoveErrorCode::from_code(1), None); // NotOwner
        assert_eq!(MoveErrorCode::from_code(2), None); // InvalidSignature
        assert_eq!(MoveErrorCode::from_code(3), None); // ReplayAttempt
        assert_eq!(MoveErrorCode::from_code(4), None); // CoinTypeMismatch
        assert_eq!(MoveErrorCode::from_code(7), None); // OwnerNotSet
        assert_eq!(MoveErrorCode::from_code(8), None); // AlreadyLinked
        assert_eq!(MoveErrorCode::from_code(9), None); // TweetAlreadyProcessed
        assert_eq!(MoveErrorCode::from_code(100), None);
    }

    // ===== get_user_reportable_message tests =====

    #[test]
    fn test_reportable_insufficient_balance() {
        let msg = r#"MoveAbort(MoveLocation { module: ModuleId { address: 0x123 }, function: 0 }, 5)"#;
        let result = get_user_reportable_message(msg);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Insufficient balance"));
    }

    #[test]
    fn test_reportable_account_exists() {
        let msg = r#"MoveAbort(MoveLocation { module: ModuleId { address: 0x123 }, function: 0 }, 0)"#;
        let result = get_user_reportable_message(msg);
        assert!(result.is_some());
        assert!(result.unwrap().contains("already have an XWallet"));
    }

    #[test]
    fn test_reportable_nft_not_found() {
        let msg = r#"MoveAbort(MoveLocation { module: ModuleId { address: 0x123 }, function: 0 }, 6)"#;
        let result = get_user_reportable_message(msg);
        assert!(result.is_some());
        assert!(result.unwrap().contains("NFT not found"));
    }

    #[test]
    fn test_reportable_insufficient_balance_pattern() {
        let msg = "Transaction failed: insufficient balance for transfer";
        let result = get_user_reportable_message(msg);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Insufficient balance"));
    }

    #[test]
    fn test_reportable_nft_not_found_pattern() {
        let msg = "NFT not found in account";
        let result = get_user_reportable_message(msg);
        assert!(result.is_some());
        assert!(result.unwrap().contains("NFT not found"));
    }

    // ===== Non-reportable errors (return None) =====

    #[test]
    fn test_not_reportable_invalid_signature() {
        let msg = r#"MoveAbort(MoveLocation { module: ModuleId { address: 0x123 }, function: 0 }, 2)"#;
        assert!(get_user_reportable_message(msg).is_none());
    }

    #[test]
    fn test_not_reportable_replay_attempt() {
        let msg = r#"MoveAbort(MoveLocation { module: ModuleId { address: 0x123 }, function: 0 }, 3)"#;
        assert!(get_user_reportable_message(msg).is_none());
    }

    #[test]
    fn test_not_reportable_timeout() {
        let msg = "Request timed out after 30 seconds";
        assert!(get_user_reportable_message(msg).is_none());
    }

    #[test]
    fn test_not_reportable_lock() {
        let msg = "Failed to acquire lock for account - another transaction may be in progress";
        assert!(get_user_reportable_message(msg).is_none());
    }

    #[test]
    fn test_not_reportable_generic() {
        let msg = "Some unknown internal error xyz";
        assert!(get_user_reportable_message(msg).is_none());
    }

    #[test]
    fn test_not_reportable_account_not_found() {
        // Generic "not found" is not reportable (only NFT not found is)
        let msg = "Account does not exist in registry";
        assert!(get_user_reportable_message(msg).is_none());
    }
}
