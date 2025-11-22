/*
/// Module: mist_protocol
module mist_protocol::mist_protocol;
*/

// For Move coding conventions, see
// https://docs.sui.io/concepts/sui-move-concepts/conventions

module mist_protocol::mist_protocol;

use mist_protocol::seal_policy::{Self, VaultEntry};
use std::string::{Self as string, String};
use sui::balance::{Self, Balance};
use sui::coin::{Self, Coin};
use sui::event;
use sui::object_bag::{Self, ObjectBag};
use sui::sui::SUI;
use sui::table::{Self, Table};
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
const E_INTENT_NOT_FOUND: u64 = 7;

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

/// Global queue for tracking pending swap intents
public struct IntentQueue has key {
    id: UID,
    pending: Table<ID, bool>,  // intent_id -> true for pending intents
}

/// Swap intent object - created when user requests a swap
/// Tickets are moved from vault into this object (locked until processed)
public struct SwapIntent has key, store {
    id: UID,
    vault_id: ID,
    locked_tickets: ObjectBag,   // Tickets moved here from vault
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    user: address,
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
    min_output_amount: u64,  // Slippage protection
    deadline: u64,           // Unix timestamp
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

/// Backend wallet address (same as in seal_policy.move)
const BACKEND_ADDRESS: address = @0x9bf64712c379154caeca62619795dbc0c839f3299518450796598a68407c2ff0;

fun init(_witness: MIST_PROTOCOL, ctx: &mut TxContext) {
    // Create liquidity pool
    let pool = LiquidityPool {
        id: object::new(ctx),
        sui_balance: balance::zero(),
        usdc_balance: balance::zero(),
        tee_authority: BACKEND_ADDRESS,  // Backend wallet, not deployer
        paused: false,
    };
    transfer::share_object(pool);

    // Create intent queue for tracking pending swaps
    let queue = IntentQueue {
        id: object::new(ctx),
        pending: table::new(ctx),
    };
    transfer::share_object(queue);

    // Create admin capability
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
/// Tickets are moved from vault into intent (locked until processed)
entry fun create_swap_intent(
    queue: &mut IntentQueue,
    vault: &mut VaultEntry,  // Now mutable - we move tickets out
    ticket_ids_in: vector<u64>,
    token_out: String,
    min_output_amount: u64,
    deadline: u64,
    ctx: &mut TxContext,
) {
    assert!(seal_policy::owner(vault) == tx_context::sender(ctx), E_NOT_OWNER);

    let vault_id = object::id(vault);
    let user = tx_context::sender(ctx);

    // Create ObjectBag to hold locked tickets
    let mut locked_tickets = object_bag::new(ctx);

    // Move tickets from vault to intent (this locks them)
    let tickets_bag = seal_policy::tickets_mut(vault);
    let mut i = 0;
    while (i < ticket_ids_in.length()) {
        let ticket_id = *ticket_ids_in.borrow(i);

        // Verify ticket exists and remove from vault
        assert!(tickets_bag.contains<u64>(ticket_id), E_TICKET_NOT_FOUND);
        let ticket = tickets_bag.remove<u64, seal_policy::EncryptedTicket>(ticket_id);

        // Add to locked tickets in intent
        locked_tickets.add<u64, seal_policy::EncryptedTicket>(ticket_id, ticket);

        i = i + 1;
    };

    // Create swap intent object with locked tickets
    let intent = SwapIntent {
        id: object::new(ctx),
        vault_id,
        locked_tickets,  // Tickets are now locked in intent
        token_out,
        min_output_amount,
        deadline,
        user,
    };

    let intent_id = object::id(&intent);

    // Add to pending queue
    queue.pending.add(intent_id, true);

    // Emit event for notification (before moving intent)
    event::emit(SwapIntentEvent {
        vault_id,
        ticket_ids_in,  // Log which tickets were locked
        token_out: intent.token_out,
        min_output_amount: intent.min_output_amount,
        deadline: intent.deadline,
        user,
    });

    // Share intent object (so TEE can read it)
    transfer::share_object(intent);
}

/// Mark swap intent as completed (called by TEE after processing)
entry fun mark_intent_completed(
    queue: &mut IntentQueue,
    intent_id: ID,
    pool: &LiquidityPool,
    ctx: &TxContext,
) {
    // Only TEE can mark intents as completed
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_AUTHORIZED);

    // Verify intent exists in queue
    assert!(queue.pending.contains(intent_id), E_INTENT_NOT_FOUND);

    // Remove from pending queue
    queue.pending.remove(intent_id);
}

/// Refund locked tickets back to vault if swap fails
/// Takes ticket_ids_to_refund to avoid needing to iterate ObjectBag
entry fun refund_intent(
    queue: &mut IntentQueue,
    intent: SwapIntent,
    vault: &mut VaultEntry,
    ticket_ids_to_refund: vector<u64>,
    pool: &LiquidityPool,
    ctx: &TxContext,
) {
    // Only TEE can refund
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_AUTHORIZED);

    let intent_id = object::id(&intent);

    // Verify intent exists in queue
    assert!(queue.pending.contains(intent_id), E_INTENT_NOT_FOUND);

    // Remove from pending queue
    queue.pending.remove(intent_id);

    // Verify this is the correct vault
    assert!(object::id(vault) == intent.vault_id, E_NOT_OWNER);

    // Unpack intent and move tickets back to vault
    let SwapIntent {
        id,
        vault_id: _,
        mut locked_tickets,
        token_out: _,
        min_output_amount: _,
        deadline: _,
        user: _,
    } = intent;

    // Get mutable reference to vault's tickets
    let vault_tickets = seal_policy::tickets_mut(vault);

    // Move all locked tickets back to vault
    let mut i = 0;
    while (i < ticket_ids_to_refund.length()) {
        let ticket_id = *ticket_ids_to_refund.borrow(i);

        assert!(locked_tickets.contains<u64>(ticket_id), E_TICKET_NOT_FOUND);
        let ticket = locked_tickets.remove<u64, seal_policy::EncryptedTicket>(ticket_id);
        vault_tickets.add<u64, seal_policy::EncryptedTicket>(ticket_id, ticket);

        i = i + 1;
    };

    // Destroy empty ObjectBag
    object_bag::destroy_empty(locked_tickets);

    // Delete intent object
    object::delete(id);
}

// ============ TEE SWAP EXECUTION ============
/// TEE executes swap and consumes locked tickets from intent
entry fun execute_swap(
    queue: &mut IntentQueue,
    intent: SwapIntent,
    vault: &mut VaultEntry,
    pool: &LiquidityPool,
    ticket_ids_consumed: vector<u64>,
    new_ticket_encrypted: vector<u8>,
    from_amount: u64,
    to_amount: u64,
    ctx: &mut TxContext,
) {
    // Only TEE can call
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_AUTHORIZED);
    assert!(!pool.paused, E_PAUSED);

    let intent_id = object::id(&intent);

    // Verify intent exists in queue
    assert!(queue.pending.contains(intent_id), E_INTENT_NOT_FOUND);

    // Verify this is the correct vault
    assert!(object::id(vault) == intent.vault_id, E_NOT_OWNER);

    // Get vault owner and next ticket ID
    let vault_owner = seal_policy::owner(vault);
    let new_ticket_id = seal_policy::next_ticket_id(vault);

    // Unpack intent to access locked tickets
    let SwapIntent {
        id,
        vault_id: _,
        mut locked_tickets,
        token_out,
        min_output_amount: _,
        deadline: _,
        user: _,
    } = intent;

    // Get token type from first locked ticket
    let first_ticket_id = *ticket_ids_consumed.borrow(0);
    assert!(locked_tickets.contains<u64>(first_ticket_id), E_TICKET_NOT_FOUND);
    let first_ticket = locked_tickets.borrow<u64, seal_policy::EncryptedTicket>(first_ticket_id);
    let token_in = seal_policy::token_type(first_ticket);

    // Remove and destroy consumed tickets from locked_tickets
    let mut i = 0;
    while (i < ticket_ids_consumed.length()) {
        let ticket_id = *ticket_ids_consumed.borrow(i);
        assert!(locked_tickets.contains<u64>(ticket_id), E_TICKET_NOT_FOUND);
        let ticket = locked_tickets.remove<u64, seal_policy::EncryptedTicket>(ticket_id);
        seal_policy::destroy_ticket(ticket);
        i = i + 1;
    };

    // Destroy empty locked_tickets bag
    object_bag::destroy_empty(locked_tickets);

    // Create new output ticket in vault
    let vault_tickets = seal_policy::tickets_mut(vault);
    let new_ticket = seal_policy::new_ticket(
        new_ticket_id,
        token_out,
        new_ticket_encrypted,
        ctx,
    );

    vault_tickets.add<u64, seal_policy::EncryptedTicket>(new_ticket_id, new_ticket);
    seal_policy::increment_ticket_id(vault);

    // Remove from pending queue
    queue.pending.remove(intent_id);

    // Delete intent object
    object::delete(id);

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
