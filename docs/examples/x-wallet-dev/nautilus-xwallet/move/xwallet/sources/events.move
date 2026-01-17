// Copyright (c) X-Wallet
// SPDX-License-Identifier: Apache-2.0

/// Events for blockchain indexing
module xwallet::events {
    use std::string::String;

    // ====== Account Events ======

    public struct AccountCreated has copy, drop {
        xid: String,
        handle: String,
        account_id: ID,
    }

    public struct WalletLinked has copy, drop {
        xid: String,
        owner_address: address,
    }

    public struct HandleUpdated has copy, drop {
        xid: String,
        old_handle: String,
        new_handle: String,
    }

    // ====== Asset Events ======

    public struct CoinDeposited has copy, drop {
        xid: String,
        coin_type: String,
        amount: u64,
    }

    public struct CoinWithdrawn has copy, drop {
        xid: String,
        coin_type: String,
        amount: u64,
    }

    public struct NftDeposited has copy, drop {
        xid: String,
        nft_id: ID,
    }

    public struct NftWithdrawn has copy, drop {
        xid: String,
        nft_id: ID,
    }

    // ====== Transfer Events ======

    public struct TransferCompleted has copy, drop {
        from_xid: String,
        to_xid: String,
        tweet_id: String,
        coin_type: String,
        amount: u64,
        timestamp: u64,
    }

    public struct NftTransferCompleted has copy, drop {
        from_xid: String,
        to_xid: String,
        nft_id: address,
        tweet_id: String,
        timestamp: u64,
    }

    // ====== Event Emission Functions ======

    public(package) fun emit_account_created(xid: String, handle: String, account_id: ID) {
        sui::event::emit(AccountCreated {
            xid,
            handle,
            account_id,
        });
    }

    public(package) fun emit_wallet_linked(xid: String, owner_address: address) {
        sui::event::emit(WalletLinked {
            xid,
            owner_address,
        });
    }

    public(package) fun emit_handle_updated(xid: String, old_handle: String, new_handle: String) {
        sui::event::emit(HandleUpdated {
            xid,
            old_handle,
            new_handle,
        });
    }

    public(package) fun emit_coin_deposited(xid: String, coin_type: String, amount: u64) {
        sui::event::emit(CoinDeposited {
            xid,
            coin_type,
            amount,
        });
    }

    public(package) fun emit_coin_withdrawn(xid: String, coin_type: String, amount: u64) {
        sui::event::emit(CoinWithdrawn {
            xid,
            coin_type,
            amount,
        });
    }

    public(package) fun emit_nft_deposited(xid: String, nft_id: ID) {
        sui::event::emit(NftDeposited {
            xid,
            nft_id,
        });
    }

    public(package) fun emit_nft_withdrawn(xid: String, nft_id: ID) {
        sui::event::emit(NftWithdrawn {
            xid,
            nft_id,
        });
    }

    public(package) fun emit_transfer_completed(
        from_xid: String,
        to_xid: String,
        tweet_id: String,
        coin_type: String,
        amount: u64,
        timestamp: u64,
    ) {
        sui::event::emit(TransferCompleted {
            from_xid,
            to_xid,
            tweet_id,
            coin_type,
            amount,
            timestamp,
        });
    }

    public(package) fun emit_nft_transfer_completed(
        from_xid: String,
        to_xid: String,
        nft_id: address,
        tweet_id: String,
        timestamp: u64,
    ) {
        sui::event::emit(NftTransferCompleted {
            from_xid,
            to_xid,
            nft_id,
            tweet_id,
            timestamp,
        });
    }
}
