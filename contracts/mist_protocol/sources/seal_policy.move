/// Seal Policy v2 for Mist Protocol
/// Allows TEE to decrypt deposit data and swap intent details
/// NO vault/owner tracking - privacy-preserving design
module mist_protocol::seal_policy_v2;

use sui::hash::blake2b256;
use enclave::enclave::Enclave;

/// Error: Caller doesn't have permission to decrypt
const ENoAccess: u64 = 0;

/// Error: Invalid namespace prefix
const EInvalidNamespace: u64 = 1;

/// The SEAL encryption namespace for v2 deposits/intents
/// All encrypted data uses this as a prefix to the encryption ID
/// Format: NAMESPACE_PREFIX (32 bytes) + random_nonce (5 bytes) = encryption_id
const NAMESPACE_PREFIX: vector<u8> = b"mist_protocol_v2_seal_namespace_";

/// Get the namespace bytes for SEAL encryption
/// This is used as the prefix for all encryption IDs in v2
public fun namespace(): vector<u8> {
    NAMESPACE_PREFIX
}

/// Check if an encryption ID has the correct namespace prefix
fun has_valid_namespace(id: vector<u8>): bool {
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

/// TEE-only seal_approve for v2
/// Only registered TEE enclave can decrypt
/// Generic over witness type W
public fun seal_approve<W: drop>(
    id: vector<u8>,
    enclave: &Enclave<W>,
    ctx: &TxContext
) {
    // Verify namespace matches
    assert!(has_valid_namespace(id), EInvalidNamespace);

    let sender = ctx.sender();
    let tee_address = pk_to_address(enclave.pk());

    // Only registered TEE can decrypt
    assert!(
        sender.to_bytes() == tee_address,
        ENoAccess
    );
}

/// Entry function for seal_approve (for direct calls)
/// NOTE: This is intentionally TEE-only. Users do NOT need to decrypt
/// because they generate the nullifier locally and store it.
entry fun seal_approve_entry<W: drop>(
    id: vector<u8>,
    enclave: &Enclave<W>,
    ctx: &TxContext
) {
    seal_approve<W>(id, enclave, ctx);
}

/// Convert enclave public key to address
/// Assumes ed25519 flag for enclave's ephemeral key
/// Derives address as blake2b_hash(flag || pk)
fun pk_to_address(pk: &vector<u8>): vector<u8> {
    let mut arr = vector[0u8]; // ed25519 flag
    arr.append(*pk);
    let hash = blake2b256(&arr);
    hash
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
    assert!(has_valid_namespace(valid_id), 0);

    let invalid_id = b"wrong_prefix_12345";
    assert!(!has_valid_namespace(invalid_id), 0);

    let short_id = b"short";
    assert!(!has_valid_namespace(short_id), 0);
}
