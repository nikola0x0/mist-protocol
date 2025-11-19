/*
/// Module: mist_protocol
module mist_protocol::mist_protocol;
*/

// For Move coding conventions, see
// https://docs.sui.io/concepts/sui-move-concepts/conventions

module mist_protocol::mist_protocol;

use mist_protocol::seal_policy::{Self, VaultEntry};
use std::option::{Self, Option};
use std::string::{Self, String};
use sui::balance::{Self, Balance};
use sui::coin::{Self, Coin};
use sui::event;
use sui::sui::SUI;
use usdc::usdc::USDC;

// ============ WITNESS (for Nautilus Enclave) ============
/// One-Time-Witness for creating Enclave capability
public struct MIST_PROTOCOL has drop {}

// ============ ERRORS ============
const E_INSUFFICIENT_BALANCE: u64 = 1;
const E_NOT_AUTHORIZED: u64 = 2;
const E_PAUSED: u64 = 3;
const E_NOT_OWNER: u64 = 4;
const E_TICKET_NOT_FOUND: u64 = 5;
const E_WRONG_TOKEN_TYPE: u64 = 6;

// ============ STRUCTS ============
public struct LiquidityPool has key {
    id: UID,
    sui_balance: Balance<SUI>,
    usdc_balance: Balance<USDC>,
    tee_authority: address,
    paused: bool,
}

public struct AdminCap has key {
    id: UID,
}

// ============ EVENTS ============
public struct TicketCreatedEvent has copy, drop {
    vault_id: ID,
    ticket_id: u64,
    token_type: String,
    amount: u64, // Real amount deposited (visible)
    user: address,
}

public struct TicketMergedEvent has copy, drop {
    vault_id: ID,
    old_ticket_ids: vector<u64>,
    new_ticket_id: u64,
    token_type: String,
    user: address,
}

public struct UnwrapEvent has copy, drop {
    user: address,
    token_type: vector<u8>,
    amount: u64,
    recipient: address,
}

/// Emitted when user requests a swap
public struct SwapIntentEvent has copy, drop {
    vault_id: ID,
    ticket_ids_in: vector<u64>,
    token_out: String,
    encrypted_intent: vector<u8>,
    user: address,
}

/// Emitted when TEE completes a swap on Cetus
public struct SwapExecutedEvent has copy, drop {
    user: address,
    from_token: vector<u8>,
    to_token: vector<u8>,
    from_amount: u64,
    to_amount: u64,
    timestamp: u64,
}

// ============ INIT ============
fun init(_witness: MIST_PROTOCOL, ctx: &mut TxContext) {
    let pool = LiquidityPool {
        id: object::new(ctx),
        sui_balance: balance::zero(),
        usdc_balance: balance::zero(),
        tee_authority: tx_context::sender(ctx),
        paused: false,
    };
    transfer::share_object(pool);

    let admin_cap = AdminCap {
        id: object::new(ctx),
    };
    transfer::transfer(admin_cap, tx_context::sender(ctx));
}

// ============ WRAP FUNCTIONS (Create Tickets) ============
/// User deposits SUI and creates encrypted ticket in vault
entry fun wrap_sui(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    payment: Coin<SUI>,
    encrypted_amount: vector<u8>,
    ctx: &mut TxContext,
) {
    assert!(!pool.paused, E_PAUSED);
    assert!(seal_policy::owner(vault) == tx_context::sender(ctx), E_NOT_OWNER);

    // Get amount
    let amount = coin::value(&payment);

    // Lock SUI in pool
    balance::join(&mut pool.sui_balance, coin::into_balance(payment));

    // Create encrypted ticket
    let ticket_id = seal_policy::next_ticket_id(vault);
    let ticket = seal_policy::new_ticket(
        ticket_id,
        string::utf8(b"SUI"),
        encrypted_amount,
        ctx,
    );

    // Add ticket to vault
    let tickets_bag = seal_policy::tickets_mut(vault);
    tickets_bag.add(ticket_id, ticket);
    seal_policy::increment_ticket_id(vault);

    // Emit event
    event::emit(TicketCreatedEvent {
        vault_id: object::id(vault),
        ticket_id,
        token_type: string::utf8(b"SUI"),
        amount,
        user: tx_context::sender(ctx),
    });
}

/// User deposits USDC and creates encrypted ticket in vault
entry fun wrap_usdc(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    payment: Coin<USDC>,
    encrypted_amount: vector<u8>,
    ctx: &mut TxContext,
) {
    assert!(!pool.paused, E_PAUSED);
    assert!(seal_policy::owner(vault) == tx_context::sender(ctx), E_NOT_OWNER);

    let amount = coin::value(&payment);

    // Lock USDC in pool
    balance::join(&mut pool.usdc_balance, coin::into_balance(payment));

    // Create encrypted ticket
    let ticket_id = seal_policy::next_ticket_id(vault);
    let ticket = seal_policy::new_ticket(
        ticket_id,
        string::utf8(b"USDC"),
        encrypted_amount,
        ctx,
    );

    // Add ticket to vault
    let tickets_bag = seal_policy::tickets_mut(vault);
    tickets_bag.add(ticket_id, ticket);
    seal_policy::increment_ticket_id(vault);

    // Emit event
    event::emit(TicketCreatedEvent {
        vault_id: object::id(vault),
        ticket_id,
        token_type: string::utf8(b"USDC"),
        amount,
        user: tx_context::sender(ctx),
    });
}

// ============ MERGE TICKETS ============
/// Merge multiple tickets into one consolidated ticket
entry fun merge_tickets(
    vault: &mut VaultEntry,
    ticket_ids: vector<u64>,
    new_encrypted_amount: vector<u8>, // Sum of all tickets, re-encrypted
    ctx: &mut TxContext,
) {
    assert!(seal_policy::owner(vault) == tx_context::sender(ctx), E_NOT_OWNER);
    assert!(ticket_ids.length() > 0, E_TICKET_NOT_FOUND);

    // Get next ticket ID before mutable borrow
    let new_ticket_id = seal_policy::next_ticket_id(vault);

    // User has decrypted all tickets, summed them, and re-encrypted
    let tickets_bag = seal_policy::tickets_mut(vault);

    // Get token type from first ticket
    let first_ticket_id = *ticket_ids.borrow(0);
    assert!(tickets_bag.contains(first_ticket_id), E_TICKET_NOT_FOUND);
    let first_ticket = tickets_bag.borrow(first_ticket_id);
    let token_type = seal_policy::token_type(first_ticket);

    // Remove all old tickets
    let mut i = 0;
    while (i < ticket_ids.length()) {
        let ticket_id = *ticket_ids.borrow(i);
        assert!(tickets_bag.contains(ticket_id), E_TICKET_NOT_FOUND);
        let old_ticket = tickets_bag.remove(ticket_id);
        seal_policy::destroy_ticket(old_ticket);
        i = i + 1;
    };

    // Create new merged ticket
    let new_ticket = seal_policy::new_ticket(
        new_ticket_id,
        token_type,
        new_encrypted_amount,
        ctx,
    );

    tickets_bag.add(new_ticket_id, new_ticket);
    seal_policy::increment_ticket_id(vault);

    // Emit event
    event::emit(TicketMergedEvent {
        vault_id: object::id(vault),
        old_ticket_ids: ticket_ids,
        new_ticket_id,
        token_type,
        user: tx_context::sender(ctx),
    });
}

// ============ SWAP INTENT FUNCTIONS ============
/// User creates swap intent using specific tickets
entry fun create_swap_intent(
    vault: &VaultEntry,
    ticket_ids_in: vector<u64>,
    token_out: String,
    encrypted_intent: vector<u8>,
    ctx: &TxContext,
) {
    assert!(seal_policy::owner(vault) == tx_context::sender(ctx), E_NOT_OWNER);

    // Verify all tickets exist in vault
    let mut i = 0;
    while (i < ticket_ids_in.length()) {
        let ticket_id = *ticket_ids_in.borrow(i);
        assert!(seal_policy::has_ticket(vault, ticket_id), E_TICKET_NOT_FOUND);
        i = i + 1;
    };

    // Emit swap intent event (TEE listens for this)
    event::emit(SwapIntentEvent {
        vault_id: object::id(vault),
        ticket_ids_in,
        token_out,
        encrypted_intent,
        user: tx_context::sender(ctx),
    });
}

// ============ TEE SWAP EXECUTION ============
/// TEE executes swap and updates vault tickets
entry fun execute_swap(
    vault: &mut VaultEntry,
    pool: &LiquidityPool,
    ticket_ids_consumed: vector<u64>,
    new_ticket_encrypted: vector<u8>,
    token_out: String,
    from_amount: u64,
    to_amount: u64,
    ctx: &mut TxContext,
) {
    // Only TEE can call
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_AUTHORIZED);
    assert!(!pool.paused, E_PAUSED);

    // Get vault owner and next ticket ID before mutable borrow
    let vault_owner = seal_policy::owner(vault);
    let new_ticket_id = seal_policy::next_ticket_id(vault);

    let tickets_bag = seal_policy::tickets_mut(vault);

    // Get token type from first ticket
    let first_ticket_id = *ticket_ids_consumed.borrow(0);
    assert!(tickets_bag.contains(first_ticket_id), E_TICKET_NOT_FOUND);
    let first_ticket = tickets_bag.borrow(first_ticket_id);
    let token_in = seal_policy::token_type(first_ticket);

    // Remove consumed tickets
    let mut i = 0;
    while (i < ticket_ids_consumed.length()) {
        let ticket_id = *ticket_ids_consumed.borrow(i);
        assert!(tickets_bag.contains(ticket_id), E_TICKET_NOT_FOUND);
        let ticket = tickets_bag.remove(ticket_id);
        seal_policy::destroy_ticket(ticket);
        i = i + 1;
    };

    // Create new output ticket
    let new_ticket = seal_policy::new_ticket(
        new_ticket_id,
        token_out,
        new_ticket_encrypted,
        ctx,
    );

    tickets_bag.add(new_ticket_id, new_ticket);
    seal_policy::increment_ticket_id(vault);

    // Emit event for transparency
    event::emit(SwapExecutedEvent {
        user: vault_owner,
        from_token: token_in.into_bytes(),
        to_token: token_out.into_bytes(),
        from_amount,
        to_amount,
        timestamp: tx_context::epoch(ctx),
    });
}

// ============ UNWRAP FUNCTIONS ============
/// User unwraps ticket to get real tokens back
entry fun unwrap_ticket(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    ticket_id: u64,
    amount: u64,
    remaining_encrypted: Option<vector<u8>>, // If Some, create new ticket with remainder
    ctx: &mut TxContext,
) {
    assert!(!pool.paused, E_PAUSED);
    assert!(seal_policy::owner(vault) == tx_context::sender(ctx), E_NOT_OWNER);

    // Get next ticket ID before mutable borrow
    let new_ticket_id = seal_policy::next_ticket_id(vault);

    let tickets_bag = seal_policy::tickets_mut(vault);
    assert!(tickets_bag.contains(ticket_id), E_TICKET_NOT_FOUND);

    // Remove ticket
    let ticket = tickets_bag.remove(ticket_id);
    let token_type = seal_policy::token_type(&ticket);

    // Verify token type and get balance
    if (token_type == string::utf8(b"SUI")) {
        assert!(balance::value(&pool.sui_balance) >= amount, E_INSUFFICIENT_BALANCE);
        let sui_to_send = coin::from_balance(
            balance::split(&mut pool.sui_balance, amount),
            ctx,
        );
        transfer::public_transfer(sui_to_send, tx_context::sender(ctx));
    } else if (token_type == string::utf8(b"USDC")) {
        assert!(balance::value(&pool.usdc_balance) >= amount, E_INSUFFICIENT_BALANCE);
        let usdc_to_send = coin::from_balance(
            balance::split(&mut pool.usdc_balance, amount),
            ctx,
        );
        transfer::public_transfer(usdc_to_send, tx_context::sender(ctx));
    } else {
        abort E_WRONG_TOKEN_TYPE
    };

    // Destroy old ticket
    seal_policy::destroy_ticket(ticket);

    // If there's a remainder, create new ticket
    if (remaining_encrypted.is_some()) {
        let new_ticket = seal_policy::new_ticket(
            new_ticket_id,
            token_type,
            remaining_encrypted.destroy_some(),
            ctx,
        );
        tickets_bag.add(new_ticket_id, new_ticket);
        seal_policy::increment_ticket_id(vault);
    };

    // Emit event
    event::emit(UnwrapEvent {
        user: tx_context::sender(ctx),
        token_type: token_type.into_bytes(),
        amount,
        recipient: tx_context::sender(ctx),
    });
}

// Convenience functions for full unwrap
entry fun unwrap_sui(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    ticket_id: u64,
    amount: u64,
    ctx: &mut TxContext,
) {
    unwrap_ticket(vault, pool, ticket_id, amount, option::none(), ctx);
}

entry fun unwrap_usdc(
    vault: &mut VaultEntry,
    pool: &mut LiquidityPool,
    ticket_id: u64,
    amount: u64,
    ctx: &mut TxContext,
) {
    unwrap_ticket(vault, pool, ticket_id, amount, option::none(), ctx);
}

// ============ ADMIN FUNCTIONS ============
/// Update TEE authority
entry fun update_tee_authority(
    pool: &mut LiquidityPool,
    _admin_cap: &AdminCap,
    new_authority: address,
) {
    pool.tee_authority = new_authority;
}

/// Pause/unpause
entry fun set_pause(pool: &mut LiquidityPool, _admin_cap: &AdminCap, paused: bool) {
    pool.paused = paused;
}
