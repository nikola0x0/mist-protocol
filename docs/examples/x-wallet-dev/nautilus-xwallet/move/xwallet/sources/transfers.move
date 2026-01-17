// Copyright (c) X-Wallet
// SPDX-License-Identifier: Apache-2.0

/// Transfer module for coin and NFT transfers between accounts
module xwallet::transfers {
    use std::ascii;
    use std::string::{Self};
    use std::type_name;
    use sui::balance::Balance;
    use xwallet::core::{Self, XWalletAccount};
    use xwallet::events;
    use enclave::enclave::Enclave;

    // ====== Coin Transfer Functions ====== 

    /// Transfer coin with signature verification
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
        // Convert tweet_id to String for table lookup
        let tweet_id_str = string::utf8(tweet_id);

        // Check tweet not already processed
        let processed_tweets = core::account_processed_tweets(from);
        assert!(!processed_tweets.contains(tweet_id_str), core::e_tweet_already_processed());

        // Verify coin type matches generic T
        let expected_type = type_name::get<T>().into_string().into_bytes();
        assert!(coin_type == expected_type, core::e_coin_type_mismatch());

        // Verify signature
        let payload = core::new_transfer_coin_payload(
            core::account_xid(from).into_bytes(),
            core::account_xid(to).into_bytes(),
            amount,
            coin_type,
            tweet_id,
        );
        let is_valid = enclave.verify_signature(
            core::transfer_coin_intent(),
            timestamp,
            payload,
            signature,
        );
        assert!(is_valid, core::e_invalid_signature());

        // Check replay
        assert!(timestamp > core::account_last_timestamp(from), core::e_replay_attempt());
        core::account_set_last_timestamp(from, timestamp);

        // Transfer balance
        transfer_balance_internal<T>(from, to, amount);

        // Mark tweet as processed
        core::account_add_processed_tweet(from, tweet_id_str);

        // Emit event
        events::emit_transfer_completed(
            core::account_xid(from),
            core::account_xid(to),
            tweet_id_str,
            type_name::get<T>().into_string().to_string(),
            amount,
            timestamp,
        );
    }

    /// Transfer coin with wallet authentication (owner signs PTB from dApp)
    public fun transfer_coin_with_wallet<T>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        amount: u64,
        ctx: &TxContext,
    ) {
        // Check owner
        assert!(core::account_owner_address(from).is_some(), core::e_owner_not_set());
        assert!(ctx.sender() == *core::account_owner_address(from).borrow(), core::e_not_owner());

        // Transfer balance
        transfer_balance_internal<T>(from, to, amount);

        // Emit event
        events::emit_transfer_completed(
            core::account_xid(from),
            core::account_xid(to),
            string::utf8(b""),
            type_name::get<T>().into_string().to_string(),
            amount,
            0,
        );
    }

    // ====== NFT Transfer Functions ======

    /// Transfer NFT with signature verification
    public fun transfer_nft<E, N: key + store>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        nft_id: address,
        tweet_id: vector<u8>,
        timestamp: u64,
        signature: &vector<u8>,
        enclave: &Enclave<E>,
        _ctx: &TxContext,
    ) {
        // Convert tweet_id to String for table lookup
        let tweet_id_str = string::utf8(tweet_id);

        // Check tweet not already processed
        let processed_tweets = core::account_processed_tweets(from);
        assert!(!processed_tweets.contains(tweet_id_str), core::e_tweet_already_processed());

        // Verify signature
        let payload = core::new_transfer_nft_payload(
            core::account_xid(from).into_bytes(),
            core::account_xid(to).into_bytes(),
            nft_id,
            tweet_id,
        );
        let is_valid = enclave.verify_signature(
            core::transfer_nft_intent(),
            timestamp,
            payload,
            signature,
        );
        assert!(is_valid, core::e_invalid_signature());

        // Check replay
        assert!(timestamp > core::account_last_timestamp(from), core::e_replay_attempt());
        core::account_set_last_timestamp(from, timestamp);

        // Transfer NFT
        let from_nfts = core::account_nfts_mut(from);
        assert!(from_nfts.contains<address>(nft_id), core::e_nft_not_found());
        let nft = from_nfts.remove<address, N>(nft_id);

        let to_nfts = core::account_nfts_mut(to);
        to_nfts.add(nft_id, nft);

        // Mark tweet as processed
        core::account_add_processed_tweet(from, tweet_id_str);

        // Emit event
        events::emit_nft_transfer_completed(
            core::account_xid(from),
            core::account_xid(to),
            nft_id,
            tweet_id_str,
            timestamp,
        );
    }

    /// Transfer NFT with wallet authentication (owner signs PTB from dApp)
    public fun transfer_nft_with_wallet<N: key + store>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        nft_id: address,
        ctx: &TxContext,
    ) {
        // Check owner
        assert!(core::account_owner_address(from).is_some(), core::e_owner_not_set());
        assert!(ctx.sender() == *core::account_owner_address(from).borrow(), core::e_not_owner());

        // Transfer NFT
        let from_nfts = core::account_nfts_mut(from);
        assert!(from_nfts.contains<address>(nft_id), core::e_nft_not_found());
        let nft = from_nfts.remove<address, N>(nft_id);

        let to_nfts = core::account_nfts_mut(to);
        to_nfts.add(nft_id, nft);

        // Emit event
        events::emit_nft_transfer_completed(
            core::account_xid(from),
            core::account_xid(to),
            nft_id,
            string::utf8(b""),  // No tweet_id for wallet-based transfers
            0,
        );
    }

    // ====== Internal Helper Functions ======

    fun transfer_balance_internal<T>(
        from: &mut XWalletAccount,
        to: &mut XWalletAccount,
        amount: u64,
    ) {
        let type_key = type_name::get<T>().into_string();

        let from_balances = core::account_balances_mut(from);
        // Check from has balance
        assert!(from_balances.contains(type_key), core::e_insufficient_balance());

        // Check sufficient balance
        let from_balance = from_balances.borrow_mut<ascii::String, Balance<T>>(type_key);
        assert!(from_balance.value() >= amount, core::e_insufficient_balance());

        // Split from source
        let transfer_balance = from_balance.split(amount);

        // Add to destination
        let to_balances = core::account_balances_mut(to);
        if (to_balances.contains(type_key)) {
            let to_balance = to_balances.borrow_mut<ascii::String, Balance<T>>(type_key);
            to_balance.join(transfer_balance);
        } else {
            to_balances.add(type_key, transfer_balance);
        };
    }
}
