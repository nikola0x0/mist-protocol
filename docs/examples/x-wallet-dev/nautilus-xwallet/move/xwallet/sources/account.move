// Copyright (c) X-Wallet
// SPDX-License-Identifier: Apache-2.0

/// Account management module for creating and managing xwallet accounts
module xwallet::account {
    use std::string::{Self, String};
    use xwallet::core::{Self, XWalletRegistry, XWalletAccount};
    use xwallet::events;
    use enclave::enclave::Enclave;

    // ====== Account Creation Functions ======

    /// Create account with enclave signature verification
    public fun init_account<T>(
        registry: &mut XWalletRegistry,
        xid: vector<u8>,
        handle: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<T>,
        ctx: &mut TxContext,
    ) {
        let xid_str = string::utf8(xid);

        // Check XID uniqueness
        assert!(!core::registry_contains_xid(registry, xid_str), core::e_xid_already_exists());

        // Verify signature
        let payload = core::new_init_account_payload(xid, handle);
        let is_valid = enclave.verify_signature(
            core::init_account_intent(),
            timestamp,
            payload,
            signature,
        );
        assert!(is_valid, core::e_invalid_signature());

        // Create account
        let mut account = core::new_account(xid_str, string::utf8(handle), ctx);
        core::account_set_last_timestamp(&mut account, timestamp);
        let account_id = core::account_id(&account);

        // Register in registry
        core::registry_add_xid(registry, xid_str, account_id);

        // Emit event
        events::emit_account_created(xid_str, string::utf8(handle), account_id);

        // Share account
        core::share_account(account);
    }

    /// Create account without signature verification (for backend auto-creation)
    /// This allows the backend to create accounts for recipients who don't have accounts yet
    public fun init_account_no_signature(
        registry: &mut XWalletRegistry,
        xid: vector<u8>,
        handle: vector<u8>,
        ctx: &mut TxContext,
    ) {
        let xid_str = string::utf8(xid);

        // Check XID uniqueness
        assert!(!core::registry_contains_xid(registry, xid_str), core::e_xid_already_exists());

        // Create account (no signature verification, no timestamp tracking)
        let account = core::new_account(xid_str, string::utf8(handle), ctx);
        let account_id = core::account_id(&account);

        // Register in registry
        core::registry_add_xid(registry, xid_str, account_id);

        // Emit event
        events::emit_account_created(xid_str, string::utf8(handle), account_id);

        // Share account
        core::share_account(account);
    }

    // ====== Wallet Linking Functions ======

    /// Link wallet with enclave signature
    public fun link_wallet<T>(
        account: &mut XWalletAccount,
        owner: address,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<T>,
    ) {
        // Verify signature
        let payload = core::new_link_wallet_payload(
            core::account_xid(account).into_bytes(),
            owner,
        );
        let is_valid = enclave.verify_signature(
            core::link_wallet_intent(),
            timestamp,
            payload,
            signature,
        );
        assert!(is_valid, core::e_invalid_signature());

        // Check replay
        assert!(timestamp > core::account_last_timestamp(account), core::e_replay_attempt());
        core::account_set_last_timestamp(account, timestamp);

        // Link wallet (allows overwriting existing wallet)
        core::account_set_owner_address(account, owner);

        // Emit event
        events::emit_wallet_linked(core::account_xid(account), owner);
    }

    // ====== Handle Update Functions ======

    /// Update handle with signature
    public fun update_handle<T>(
        account: &mut XWalletAccount,
        new_handle: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<T>,
    ) {
        // Verify signature
        let payload = core::new_update_handle_payload(
            core::account_xid(account).into_bytes(),
            new_handle,
        );
        let is_valid = enclave.verify_signature(
            core::update_handle_intent(),
            timestamp,
            payload,
            signature,
        );
        assert!(is_valid, core::e_invalid_signature());

        // Check replay
        assert!(timestamp > core::account_last_timestamp(account), core::e_replay_attempt());
        core::account_set_last_timestamp(account, timestamp);

        // Update handle
        let old_handle = core::account_handle(account);
        let new_handle_str = string::utf8(new_handle);
        core::account_set_handle(account, new_handle_str);

        // Emit event
        events::emit_handle_updated(core::account_xid(account), old_handle, new_handle_str);
    }

    // ====== View Functions ======

    public fun get_account_id(registry: &XWalletRegistry, xid: String): Option<ID> {
        if (core::registry_contains_xid(registry, xid)) {
            option::some(core::registry_get_account_id(registry, xid))
        } else {
            option::none()
        }
    }
}
