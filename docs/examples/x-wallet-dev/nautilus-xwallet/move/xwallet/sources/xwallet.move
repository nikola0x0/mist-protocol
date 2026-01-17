// Copyright (c) X-Wallet
// SPDX-License-Identifier: Apache-2.0

/// Main xwallet module that provides wrapper functions for backward compatibility
module xwallet::xwallet {
    use std::string::String;
    use sui::coin::Coin;
    use xwallet::core::{Self, XWalletRegistry, XWalletAccount};
    use xwallet::account;
    use xwallet::assets;
    use xwallet::transfers;
    use enclave::enclave::Enclave;

    // Error code constants (for test expected_failure attributes)
    #[allow(unused_const)]
    const EXidAlreadyExists: u64 = 0;
    #[allow(unused_const)]
    const ENotOwner: u64 = 1;
    #[allow(unused_const)]
    const EInvalidSignature: u64 = 2;
    #[allow(unused_const)]
    const EReplayAttempt: u64 = 3;
    #[allow(unused_const)]
    const ECoinTypeMismatch: u64 = 4;
    #[allow(unused_const)]
    const EInsufficientBalance: u64 = 5;
    #[allow(unused_const)]
    const ENftNotFound: u64 = 6;
    #[allow(unused_const)]
    const EOwnerNotSet: u64 = 7;
    #[allow(unused_const)]
    const EAlreadyLinked: u64 = 8;
    #[allow(unused_const)]
    const ETweetAlreadyProcessed: u64 = 9;

    // ====== Account Management Wrappers ======

    public fun init_account<T>(
        registry: &mut XWalletRegistry,
        xid: vector<u8>,
        handle: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<T>,
        ctx: &mut TxContext,
    ) {
        account::init_account(registry, xid, handle, timestamp, signature, enclave, ctx);
    }

    public fun link_wallet<T>(
        account: &mut XWalletAccount,
        owner: address,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<T>,
    ) {
        account::link_wallet(account, owner, timestamp, signature, enclave);
    }

    public fun update_handle<T>(
        account: &mut XWalletAccount,
        new_handle: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<T>,
    ) {
        account::update_handle(account, new_handle, timestamp, signature, enclave);
    }

    // ====== Asset Management Wrappers ======

    public fun deposit_coin<T>(
        account: &mut XWalletAccount,
        coin: Coin<T>,
        ctx: &TxContext,
    ) {
        assets::deposit_coin(account, coin, ctx);
    }

    public fun withdraw_coin<T>(
        account: &mut XWalletAccount,
        amount: u64,
        ctx: &mut TxContext,
    ): Coin<T> {
        assets::withdraw_coin(account, amount, ctx)
    }

    public fun get_balance<T>(account: &XWalletAccount): u64 {
        assets::get_balance<T>(account)
    }

    public fun deposit_nft<T: key + store>(
        account: &mut XWalletAccount,
        nft: T,
        ctx: &TxContext,
    ) {
        assets::deposit_nft(account, nft, ctx);
    }

    public fun withdraw_nft<T: key + store>(
        account: &mut XWalletAccount,
        nft_id: address,
        ctx: &TxContext,
    ): T {
        assets::withdraw_nft(account, nft_id, ctx)
    }

    // ====== Transfer Function Wrappers ======

    public fun transfer_coin<T, E>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        amount: u64,
        coin_type: vector<u8>,
        tweet_id: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<E>,
    ) {
        transfers::transfer_coin<T, E>(from, to, amount, coin_type, tweet_id, timestamp, signature, enclave);
    }

    public fun transfer_coin_with_wallet<T>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        amount: u64,
        ctx: &TxContext,
    ) {
        transfers::transfer_coin_with_wallet<T>(from, to, amount, ctx);
    }

    public fun transfer_nft<E, N: key + store>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        nft_id: address,
        tweet_id: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<E>,
        ctx: &TxContext,
    ) {
        transfers::transfer_nft<E, N>(from, to, nft_id, tweet_id, timestamp, signature, enclave, ctx);
    }

    public fun transfer_nft_with_wallet<N: key + store>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        nft_id: address,
        ctx: &TxContext,
    ) {
        transfers::transfer_nft_with_wallet<N>(from, to, nft_id, ctx);
    }

    // ====== View Function Wrappers ======

    public fun xid(account: &XWalletAccount): String {
        core::account_xid(account)
    }

    public fun handle(account: &XWalletAccount): String {
        core::account_handle(account)
    }

    public fun owner_address(account: &XWalletAccount): Option<address> {
        *core::account_owner_address(account)
    }

    public fun last_timestamp(account: &XWalletAccount): u64 {
        core::account_last_timestamp(account)
    }

    public fun get_account_id(registry: &XWalletRegistry, xid: String): Option<ID> {
        account::get_account_id(registry, xid)
    }

    // ====== Test-Only Functions ======

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        core::init_for_testing(ctx);
    }
}
