/// Seal Policy v2 for Mist Protocol
/// Supports two decryption modes:
/// 1. TEE decryption - for backend to process swap intents
/// 2. User decryption - for users to decrypt their own encrypted data
module mist_protocol::seal_policy;

/// Error: Caller doesn't have permission to decrypt
const ENoAccess: u64 = 0;

/// Error: Invalid namespace prefix
const EInvalidNamespace: u64 = 1;

/// Error: Invalid encryption ID length
const EInvalidIdLength: u64 = 2;

/// The SEAL encryption namespace for Mist Protocol v2
/// All encrypted data uses this as a prefix to the encryption ID
const NAMESPACE_PREFIX: vector<u8> = b"mist_protocol_v2_seal_namespace_";

/// TEE backend address - only this address can decrypt via seal_approve_tee
const BACKEND_ADDRESS: address =
    @0x9bf64712c379154caeca62619795dbc0c839f3299518450796598a68407c2ff0;

/// Encryption ID format for TEE decryption:
/// NAMESPACE_PREFIX (32 bytes) + nonce (5 bytes) = 37 bytes
const TEE_ID_LENGTH: u64 = 37;

/// Encryption ID format for user decryption:
/// NAMESPACE_PREFIX (32 bytes) + user_address (32 bytes) + nonce (5 bytes) = 69 bytes
const USER_ID_LENGTH: u64 = 69;

/// Get the namespace bytes for SEAL encryption
public fun namespace(): vector<u8> {
    NAMESPACE_PREFIX
}

/// Check if an encryption ID has the correct namespace prefix
fun has_valid_namespace(id: &vector<u8>): bool {
    let ns_len = NAMESPACE_PREFIX.length();

    if (id.length() < ns_len) {
        return false
    };

    let mut i = 0;
    while (i < ns_len) {
        if (id[i] != NAMESPACE_PREFIX[i]) {
            return false
        };
        i = i + 1;
    };

    true
}

/// Extract address from encryption ID (bytes 32-63)
/// Used for user decryption to verify ownership
fun extract_address_from_id(id: &vector<u8>): address {
    let ns_len = NAMESPACE_PREFIX.length(); // 32

    let mut addr_bytes: vector<u8> = vector[];
    let mut i = 0;
    while (i < 32) {
        addr_bytes.push_back(id[ns_len + i]);
        i = i + 1;
    };

    sui::address::from_bytes(addr_bytes)
}

/// TEE seal_approve - allows backend to decrypt swap intent details
///
/// Encryption ID format: NAMESPACE_PREFIX (32) + nonce (5) = 37 bytes
/// Only the hardcoded TEE address can decrypt.
///
/// SEAL servers will simulate this call to verify authorization.
/// If it succeeds (doesn't abort), they release decryption keys.
public fun seal_approve_tee(id: vector<u8>, ctx: &TxContext) {
    // Verify namespace matches
    assert!(has_valid_namespace(&id), EInvalidNamespace);

    // Verify correct ID length for TEE format
    assert!(id.length() == TEE_ID_LENGTH, EInvalidIdLength);

    // Only TEE backend can decrypt
    assert!(ctx.sender() == BACKEND_ADDRESS, ENoAccess);
}

/// Entry function for seal_approve_tee
entry fun seal_approve_tee_entry(id: vector<u8>, ctx: &TxContext) {
    seal_approve_tee(id, ctx);
}

/// User seal_approve - allows users to decrypt their own encrypted data
///
/// Encryption ID format: NAMESPACE_PREFIX (32) + user_address (32) + nonce (5) = 69 bytes
/// Only the user whose address is embedded in the ID can decrypt.
///
/// This is used for:
/// - Users viewing their encrypted deposit information
/// - Users decrypting their own private data
public fun seal_approve_user(id: vector<u8>, ctx: &TxContext) {
    // Verify namespace matches
    assert!(has_valid_namespace(&id), EInvalidNamespace);

    // Verify correct ID length for user format
    assert!(id.length() == USER_ID_LENGTH, EInvalidIdLength);

    // Extract the owner address from the ID
    let owner = extract_address_from_id(&id);

    // Only the owner can decrypt their own data
    assert!(ctx.sender() == owner, ENoAccess);
}

/// Entry function for seal_approve_user
entry fun seal_approve_user_entry(id: vector<u8>, ctx: &TxContext) {
    seal_approve_user(id, ctx);
}

/// Get the TEE backend address
public fun tee_address(): address {
    BACKEND_ADDRESS
}

/// Check if an address is the authorized TEE
public fun is_tee(addr: address): bool {
    addr == BACKEND_ADDRESS
}

/// Get the expected ID length for TEE encryption
public fun tee_id_length(): u64 {
    TEE_ID_LENGTH
}

/// Get the expected ID length for user encryption
public fun user_id_length(): u64 {
    USER_ID_LENGTH
}

#[test]
fun test_namespace() {
    let ns = namespace();
    assert!(ns.length() == 32, 0);
}

#[test]
fun test_valid_namespace() {
    let mut valid_id = NAMESPACE_PREFIX;
    valid_id.append(b"12345"); // 5 byte nonce
    assert!(has_valid_namespace(&valid_id), 0);

    let invalid_id = b"wrong_prefix_12345";
    assert!(!has_valid_namespace(&invalid_id), 0);

    let short_id = b"short";
    assert!(!has_valid_namespace(&short_id), 0);
}

#[test]
fun test_tee_id_length() {
    // NAMESPACE_PREFIX (32) + nonce (5) = 37
    let mut tee_id = NAMESPACE_PREFIX;
    tee_id.append(b"12345");
    assert!(tee_id.length() == TEE_ID_LENGTH, 0);
}

#[test]
fun test_user_id_length() {
    // NAMESPACE_PREFIX (32) + address (32) + nonce (5) = 69
    let mut user_id = NAMESPACE_PREFIX;
    // Add 32 bytes for address
    let mut i = 0;
    while (i < 32) {
        user_id.push_back(0u8);
        i = i + 1;
    };
    // Add 5 bytes for nonce
    user_id.append(b"12345");
    assert!(user_id.length() == USER_ID_LENGTH, 0);
}
