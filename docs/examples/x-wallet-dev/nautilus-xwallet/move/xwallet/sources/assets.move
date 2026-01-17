// Copyright (c) X-Wallet
// SPDX-License-Identifier: Apache-2.0

/// Asset management module for coin and NFT operations
module xwallet::assets {
    use std::ascii;
    use std::type_name;
    use sui::balance::Balance;
    use sui::coin::Coin;
    use xwallet::core::{Self, XWalletAccount};
    use xwallet::events;

    // ====== Coin Functions ======

    /// Deposit coin into account (anyone can deposit)
    public fun deposit_coin<T>(
        account: &mut XWalletAccount,
        coin: Coin<T>,
        _ctx: &TxContext,
    ) {
        // No owner check - anyone can deposit to any account
        let type_key = type_name::get<T>().into_string();
        let amount = coin.value();
        let balance = coin.into_balance();

        let balances = core::account_balances_mut(account);
        if (balances.contains(type_key)) {
            let existing = balances.borrow_mut<ascii::String, Balance<T>>(type_key);
            existing.join(balance);
        } else {
            balances.add(type_key, balance);
        };

        // Emit event
        events::emit_coin_deposited(
            core::account_xid(account),
            type_key.to_string(),
            amount,
        );
    }

    /// Withdraw coin from account (owner only)
    public fun withdraw_coin<T>(
        account: &mut XWalletAccount,
        amount: u64,
        ctx: &mut TxContext,
    ): Coin<T> {
        // Check owner
        assert!(core::account_owner_address(account).is_some(), core::e_owner_not_set());
        assert!(ctx.sender() == *core::account_owner_address(account).borrow(), core::e_not_owner());

        let type_key = type_name::get<T>().into_string();
        let balances = core::account_balances_mut(account);

        // Check balance exists and is sufficient
        assert!(balances.contains(type_key), core::e_insufficient_balance());
        let balance = balances.borrow_mut<ascii::String, Balance<T>>(type_key);
        assert!(balance.value() >= amount, core::e_insufficient_balance());

        let coin = balance.split(amount).into_coin(ctx);

        // Emit event
        events::emit_coin_withdrawn(
            core::account_xid(account),
            type_key.to_string(),
            amount,
        );

        coin
    }

    /// Get balance for a coin type
    public fun get_balance<T>(account: &XWalletAccount): u64 {
        let type_key = type_name::get<T>().into_string();
        let balances = core::account_balances(account);
        if (balances.contains(type_key)) {
            balances.borrow<ascii::String, Balance<T>>(type_key).value()
        } else {
            0
        }
    }

    // ====== NFT Functions ======

    /// Deposit NFT into account (anyone can deposit)
    public fun deposit_nft<T: key + store>(
        account: &mut XWalletAccount,
        nft: T,
        _ctx: &TxContext,
    ) {
        // No owner check - anyone can deposit NFT to any account
        let nft_id = object::id(&nft);
        let nft_addr = object::id_to_address(&nft_id);
        let nfts = core::account_nfts_mut(account);
        nfts.add(nft_addr, nft);

        // Emit event
        events::emit_nft_deposited(core::account_xid(account), nft_id);
    }

    /// Withdraw NFT from account (owner only)
    public fun withdraw_nft<T: key + store>(
        account: &mut XWalletAccount,
        nft_id: address,
        ctx: &TxContext,
    ): T {
        // Check owner
        assert!(core::account_owner_address(account).is_some(), core::e_owner_not_set());
        assert!(ctx.sender() == *core::account_owner_address(account).borrow(), core::e_not_owner());

        let nfts = core::account_nfts_mut(account);
        // Check NFT exists
        assert!(nfts.contains<address>(nft_id), core::e_nft_not_found());

        let nft = nfts.remove<address, T>(nft_id);
        let nft_obj_id = object::id(&nft);

        // Emit event
        events::emit_nft_withdrawn(core::account_xid(account), nft_obj_id);

        nft
    }
}
