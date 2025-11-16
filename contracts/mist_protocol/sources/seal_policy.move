// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Seal Policy for Mist Protocol
/// This module implements the seal_approve function that grants permission
/// for BOTH the user AND the Nautilus TEE to decrypt vault balances using Seal threshold encryption.

module mist_protocol::seal_policy {
    use enclave::enclave::Enclave;
    use mist_protocol::mist_protocol::MIST_PROTOCOL;
    use sui::hash::blake2b256;

    /// Error: Caller doesn't have permission to approve
    const ENoAccess: u64 = 0;

    /// Error: Invalid namespace prefix
    const EInvalidNamespace: u64 = 1;

    /// Vault entry - namespace for user's encrypted balances
    /// Shared object that both user and TEE can reference
    public struct VaultEntry has key {
        id: UID,
        owner: address,  // User who owns this vault
    }

    /// Create a vault for the user
    public fun create_vault(ctx: &mut TxContext): VaultEntry {
        VaultEntry {
            id: object::new(ctx),
            owner: ctx.sender(),
        }
    }

    /// Entry function to create vault
    entry fun create_vault_entry(ctx: &mut TxContext) {
        let vault = create_vault(ctx);
        transfer::share_object(vault);  // Shared so TEE can access
    }

    /// Get the namespace bytes for SEAL encryption
    /// This is used as the prefix for encryption IDs
    public fun namespace(vault: &VaultEntry): vector<u8> {
        vault.id.to_bytes()
    }

    /// Check if an encryption ID has the correct vault namespace prefix
    fun has_valid_namespace(id: vector<u8>, vault: &VaultEntry): bool {
        let namespace_bytes = namespace(vault);
        let ns_len = namespace_bytes.length();

        if (id.length() < ns_len) {
            return false
        };

        let mut i = 0;
        while (i < ns_len) {
            if (id[i] != namespace_bytes[i]) {
                return false
            };
            i = i + 1;
        };

        true
    }

    /// User-only seal_approve (no enclave needed)
    /// Allows vault owner to decrypt their own balances
    entry fun seal_approve_user(
        id: vector<u8>,
        vault: &VaultEntry,
        ctx: &TxContext
    ) {
        // Verify namespace matches
        assert!(has_valid_namespace(id, vault), EInvalidNamespace);

        // Only vault owner can decrypt
        assert!(
            ctx.sender().to_bytes() == vault.owner.to_bytes(),
            ENoAccess
        );
    }

    /// TEE seal_approve (requires enclave)
    /// Allows registered TEE to decrypt for swap execution
    entry fun seal_approve_tee(
        id: vector<u8>,
        vault: &VaultEntry,
        enclave: &Enclave<MIST_PROTOCOL>,
        ctx: &TxContext
    ) {
        // Verify namespace matches
        assert!(has_valid_namespace(id, vault), EInvalidNamespace);

        let sender = ctx.sender();
        let tee_address = pk_to_address(enclave.pk());

        // Only registered TEE can decrypt
        assert!(
            sender.to_bytes() == tee_address,
            ENoAccess
        );
    }

    /// Combined seal_approve (user OR TEE)
    /// Kept for backwards compatibility
    entry fun seal_approve(
        id: vector<u8>,
        vault: &VaultEntry,
        enclave: &Enclave<MIST_PROTOCOL>,
        ctx: &TxContext
    ) {
        // Verify namespace matches
        assert!(has_valid_namespace(id, vault), EInvalidNamespace);

        let sender = ctx.sender();
        let tee_address = pk_to_address(enclave.pk());

        // Allow if sender is EITHER:
        // 1. The vault owner (user), OR
        // 2. The registered TEE
        assert!(
            sender.to_bytes() == vault.owner.to_bytes() ||
            sender.to_bytes() == tee_address,
            ENoAccess
        );
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
    fun test_pk_to_address() {
        let eph_pk = x"5c38d3668c45ff891766ee99bd3522ae48d9771dc77e8a6ac9f0bde6c3a2ca48";
        let expected_bytes = x"29287d8584fb5b71b8d62e7224b867207d205fb61d42b7cce0deef95bf4e8202";
        assert!(pk_to_address(&eph_pk) == expected_bytes, ENoAccess);
    }
}
