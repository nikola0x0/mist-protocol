// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Seal Policy for Mist Protocol
/// This module implements the seal_approve function that grants permission
/// for BOTH the user AND the Nautilus TEE to decrypt vault balances using Seal threshold encryption.

module mist_protocol::seal_policy {
    use sui::hash::blake2b256;
    use sui::event;
    use std::string::String;
    use sui::object_bag::{Self, ObjectBag};
    use enclave::enclave::Enclave;

    /// Error: Caller doesn't have permission to approve
    const ENoAccess: u64 = 0;

    /// Error: Invalid namespace prefix
    const EInvalidNamespace: u64 = 1;

    /// Error: Not vault owner
    const ENotOwner: u64 = 2;

    /// Error: Ticket not found
    const ETicketNotFound: u64 = 3;

    /// Error: User already has a vault
    const EAlreadyHasVault: u64 = 4;

    /// Registry of user's vaults (owned object for easy discovery)
    public struct VaultRegistry has key, store {
        id: UID,
        vault_ids: vector<ID>,  // List of vault IDs owned by this user
    }

    /// Event: Vault created
    public struct VaultCreatedEvent has copy, drop {
        vault_id: ID,
        owner: address,
    }

    /// Vault entry - container for user's encrypted ticket balances
    /// Shared object that both user and TEE can reference
    public struct VaultEntry has key {
        id: UID,
        owner: address,  // User who owns this vault
        tickets: ObjectBag,  // Collection of EncryptedTicket objects
        next_ticket_id: u64,  // Sequential ID for tickets
    }

    /// Encrypted ticket representing a token balance
    /// Stored inside the vault's ObjectBag
    public struct EncryptedTicket has key, store {
        id: UID,
        ticket_id: u64,  // Sequential ID for easy tracking
        token_type: String,  // "SUI", "USDC", etc.
        encrypted_amount: vector<u8>,  // SEAL encrypted balance
    }

    /// Create a vault for the user
    public fun create_vault(ctx: &mut TxContext): VaultEntry {
        VaultEntry {
            id: object::new(ctx),
            owner: ctx.sender(),
            tickets: object_bag::new(ctx),
            next_ticket_id: 0,
        }
    }

    /// Get vault owner
    public fun owner(vault: &VaultEntry): address {
        vault.owner
    }

    /// Get tickets bag reference (for reading)
    public fun tickets(vault: &VaultEntry): &ObjectBag {
        &vault.tickets
    }

    /// Get mutable tickets bag reference (for mist_protocol module)
    public(package) fun tickets_mut(vault: &mut VaultEntry): &mut ObjectBag {
        &mut vault.tickets
    }

    /// Get next ticket ID
    public fun next_ticket_id(vault: &VaultEntry): u64 {
        vault.next_ticket_id
    }

    /// Increment next ticket ID (for mist_protocol module)
    public(package) fun increment_ticket_id(vault: &mut VaultEntry) {
        vault.next_ticket_id = vault.next_ticket_id + 1;
    }

    /// Check if vault owns a specific ticket
    public fun has_ticket(vault: &VaultEntry, ticket_id: u64): bool {
        vault.tickets.contains(ticket_id)
    }

    /// Create a new encrypted ticket (for mist_protocol module)
    public(package) fun new_ticket(
        ticket_id: u64,
        token_type: String,
        encrypted_amount: vector<u8>,
        ctx: &mut TxContext
    ): EncryptedTicket {
        EncryptedTicket {
            id: object::new(ctx),
            ticket_id,
            token_type,
            encrypted_amount,
        }
    }

    /// Get ticket fields (for reading)
    public fun ticket_id(ticket: &EncryptedTicket): u64 {
        ticket.ticket_id
    }

    public fun token_type(ticket: &EncryptedTicket): String {
        ticket.token_type
    }

    public fun encrypted_amount(ticket: &EncryptedTicket): vector<u8> {
        ticket.encrypted_amount
    }

    /// Destroy a ticket (for mist_protocol module)
    public(package) fun destroy_ticket(ticket: EncryptedTicket) {
        let EncryptedTicket { id, ticket_id: _, token_type: _, encrypted_amount: _ } = ticket;
        object::delete(id);
    }

    /// Create a new vault registry for the user
    fun new_registry(ctx: &mut TxContext): VaultRegistry {
        VaultRegistry {
            id: object::new(ctx),
            vault_ids: vector::empty(),
        }
    }

    /// Get vault IDs from registry
    public fun registry_vault_ids(registry: &VaultRegistry): vector<ID> {
        registry.vault_ids
    }

    /// Entry function to create vault with registry
    /// Creates both a shared VaultEntry and an owned VaultRegistry
    entry fun create_vault_entry(ctx: &mut TxContext) {
        let vault = create_vault(ctx);
        let vault_id = object::id(&vault);

        // Create registry for this user
        let mut registry = new_registry(ctx);
        registry.vault_ids.push_back(vault_id);

        // Emit event for indexing
        event::emit(VaultCreatedEvent {
            vault_id,
            owner: ctx.sender(),
        });

        // Transfer registry to user (owned)
        transfer::transfer(registry, ctx.sender());

        // Share vault (so TEE can access)
        transfer::share_object(vault);
    }

    /// Entry function to add additional vault to existing registry
    entry fun add_vault_to_registry(
        registry: &mut VaultRegistry,
        ctx: &mut TxContext
    ) {
        let vault = create_vault(ctx);
        let vault_id = object::id(&vault);

        // Add to registry
        registry.vault_ids.push_back(vault_id);

        // Emit event
        event::emit(VaultCreatedEvent {
            vault_id,
            owner: ctx.sender(),
        });

        // Share vault
        transfer::share_object(vault);
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
    /// Generic over witness type W
    public entry fun seal_approve_tee<W: drop>(
        id: vector<u8>,
        vault: &VaultEntry,
        enclave: &Enclave<W>,
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
    /// Generic over witness type W
    public entry fun seal_approve<W: drop>(
        id: vector<u8>,
        vault: &VaultEntry,
        enclave: &Enclave<W>,
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
