/// Mist Protocol v2: Privacy-Preserving DEX Swaps
/// Nullifier-based privacy - like Tornado Cash for Sui
module mist_protocol::mist_protocol;

use sui::balance::{Self, Balance};
use sui::coin::{Self, Coin};
use sui::event;
use sui::sui::SUI;
use sui::table::{Self, Table};

// ============ ERRORS ============
const E_NULLIFIER_SPENT: u64 = 1;
const E_NOT_TEE: u64 = 2;
const E_INSUFFICIENT_BALANCE: u64 = 3;
const E_PAUSED: u64 = 4;
const E_DEADLINE_PASSED: u64 = 5;

// ============ STRUCTS ============

/// Deposit object - NO OWNER FIELD for privacy
/// Anyone can see the deposit exists, but can't link it to a swap
public struct Deposit has key, store {
    id: UID,
    encrypted_data: vector<u8>, // SEAL(amount, nullifier) - only TEE can decrypt
    token_type: vector<u8>,     // b"SUI"
    amount: u64,                // Visible (from deposit tx anyway)
}

/// Registry of spent nullifiers - prevents double-spend
public struct NullifierRegistry has key {
    id: UID,
    spent: Table<vector<u8>, bool>, // nullifier -> spent
}

/// Swap intent - NO DEPOSIT REFERENCE for privacy
/// Contains only encrypted blob with nullifier, amounts, stealth addresses
public struct SwapIntent has key {
    id: UID,
    encrypted_details: vector<u8>, // SEAL(nullifier, input_amount, output_stealth, remainder_stealth)
    token_in: vector<u8>,          // b"SUI"
    token_out: vector<u8>,         // b"SUI"
    deadline: u64,                 // Unix timestamp
}

/// Liquidity pool holding all deposited tokens
public struct LiquidityPool has key {
    id: UID,
    sui_balance: Balance<SUI>,
    tee_authority: address,
    paused: bool,
}

/// Admin capability
public struct AdminCap has key {
    id: UID,
}

// ============ EVENTS ============

/// Emitted when user creates a deposit
/// Observer sees: wallet, amount, token type
/// Observer does NOT see: nullifier (encrypted)
public struct DepositCreatedEvent has copy, drop {
    deposit_id: ID,
    token_type: vector<u8>,
    amount: u64,
    // NOTE: No user field! Privacy-preserving.
    // The deposit tx itself reveals the depositor, but we don't store it.
}

/// Emitted when user creates swap intent
/// Observer sees: encrypted blob only
/// Observer does NOT see: nullifier, amounts, addresses
public struct SwapIntentCreatedEvent has copy, drop {
    intent_id: ID,
    token_in: vector<u8>,
    token_out: vector<u8>,
    deadline: u64,
    // NOTE: No user field, no amounts, no deposit reference!
}

/// Emitted when TEE executes swap
/// Observer sees: nullifier spent, stealth outputs
/// Observer CANNOT link: which deposit the nullifier came from
public struct SwapExecutedEvent has copy, drop {
    nullifier_hash: vector<u8>, // Hash of nullifier (not the nullifier itself)
    output_stealth: address,
    remainder_stealth: address,
    output_amount: u64,
    remainder_amount: u64,
}

/// Emitted when deposit is marked as consumed (optional for cleanup)
public struct DepositConsumedEvent has copy, drop {
    deposit_id: ID,
}

// ============ BACKEND ADDRESS ============
/// TEE authority address - only this address can execute swaps
const BACKEND_ADDRESS: address =
    @0x9bf64712c379154caeca62619795dbc0c839f3299518450796598a68407c2ff0;

// ============ INIT ============
fun init(ctx: &mut TxContext) {
    // Create liquidity pool
    let pool = LiquidityPool {
        id: object::new(ctx),
        sui_balance: balance::zero(),
        tee_authority: BACKEND_ADDRESS,
        paused: false,
    };
    transfer::share_object(pool);

    // Create nullifier registry
    let registry = NullifierRegistry {
        id: object::new(ctx),
        spent: table::new(ctx),
    };
    transfer::share_object(registry);

    // Create admin capability
    let admin_cap = AdminCap {
        id: object::new(ctx),
    };
    transfer::transfer(admin_cap, tx_context::sender(ctx));
}

// ============ DEPOSIT FUNCTIONS ============

/// User deposits SUI and creates a Deposit object
/// The deposit has NO owner field - privacy!
/// User stores nullifier locally (must backup!)
entry fun deposit_sui(
    pool: &mut LiquidityPool,
    payment: Coin<SUI>,
    encrypted_data: vector<u8>, // SEAL(amount, nullifier)
    ctx: &mut TxContext,
) {
    assert!(!pool.paused, E_PAUSED);

    let amount = coin::value(&payment);

    // Lock SUI in pool
    balance::join(&mut pool.sui_balance, coin::into_balance(payment));

    // Create deposit object (NO OWNER!)
    let deposit = Deposit {
        id: object::new(ctx),
        encrypted_data,
        token_type: b"SUI",
        amount,
    };

    let deposit_id = object::id(&deposit);

    // Emit event (no user address!)
    event::emit(DepositCreatedEvent {
        deposit_id,
        token_type: b"SUI",
        amount,
    });

    // Share deposit object (so TEE can read and scan)
    transfer::share_object(deposit);
}

// ============ SWAP INTENT FUNCTIONS ============

/// User creates swap intent - NO DEPOSIT REFERENCE!
/// Contains only encrypted blob with nullifier, amounts, stealth addresses
/// TEE will scan all deposits to find matching nullifier
entry fun create_swap_intent(
    encrypted_details: vector<u8>, // SEAL(nullifier, input_amount, output_stealth, remainder_stealth)
    token_in: vector<u8>,
    token_out: vector<u8>,
    deadline: u64,
    ctx: &mut TxContext,
) {
    let intent = SwapIntent {
        id: object::new(ctx),
        encrypted_details,
        token_in,
        token_out,
        deadline,
    };

    let intent_id = object::id(&intent);

    // Emit event (no user address, no amounts!)
    event::emit(SwapIntentCreatedEvent {
        intent_id,
        token_in,
        token_out,
        deadline,
    });

    // Share intent object (so TEE can read)
    transfer::share_object(intent);
}

// ============ TEE EXECUTION ============

/// TEE withdraws SUI from pool for external DEX swap
/// Marks nullifier as spent, returns SUI to TEE
/// TEE then swaps on Cetus/FlowX and sends to stealth address
public fun withdraw_for_swap(
    registry: &mut NullifierRegistry,
    pool: &mut LiquidityPool,
    intent: SwapIntent,
    nullifier: vector<u8>,       // Revealed by TEE after decryption
    withdraw_amount: u64,        // Amount to withdraw for swap
    ctx: &mut TxContext,
): Coin<SUI> {
    // Only TEE can execute
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_TEE);
    assert!(!pool.paused, E_PAUSED);

    // Check deadline
    assert!(tx_context::epoch_timestamp_ms(ctx) <= intent.deadline, E_DEADLINE_PASSED);

    // Verify nullifier not already spent (double-spend protection)
    assert!(!table::contains(&registry.spent, nullifier), E_NULLIFIER_SPENT);

    // Mark nullifier as spent
    table::add(&mut registry.spent, nullifier, true);

    // Verify pool has enough balance
    assert!(balance::value(&pool.sui_balance) >= withdraw_amount, E_INSUFFICIENT_BALANCE);

    // Hash nullifier for event (don't reveal raw nullifier in events)
    let nullifier_hash = sui::hash::blake2b256(&nullifier);

    // Emit event (output details will be in the transfer tx)
    event::emit(SwapExecutedEvent {
        nullifier_hash,
        output_stealth: @0x0, // TEE will handle transfer
        remainder_stealth: @0x0,
        output_amount: withdraw_amount,
        remainder_amount: 0,
    });

    // Cleanup intent
    let SwapIntent { id, encrypted_details: _, token_in: _, token_out: _, deadline: _ } = intent;
    object::delete(id);

    // Return SUI coin to TEE for external swap
    coin::from_balance(balance::split(&mut pool.sui_balance, withdraw_amount), ctx)
}

/// TEE executes swap with SUI output (legacy - for SUI→SUI privacy mixer)
/// Marks nullifier as spent and sends to stealth addresses
/// NO deposit ID passed - TEE already scanned all deposits offchain
entry fun execute_swap(
    registry: &mut NullifierRegistry,
    pool: &mut LiquidityPool,
    intent: SwapIntent,
    nullifier: vector<u8>,       // Revealed by TEE after decryption
    output_amount: u64,          // After swap (from Cetus)
    output_stealth: address,     // One-time address
    remainder_amount: u64,       // Leftover from deposit
    remainder_stealth: address,  // One-time address for remainder
    ctx: &mut TxContext,
) {
    // Only TEE can execute
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_TEE);
    assert!(!pool.paused, E_PAUSED);

    // Check deadline
    assert!(tx_context::epoch_timestamp_ms(ctx) <= intent.deadline, E_DEADLINE_PASSED);

    // Verify nullifier not already spent (double-spend protection)
    assert!(!table::contains(&registry.spent, nullifier), E_NULLIFIER_SPENT);

    // Mark nullifier as spent
    table::add(&mut registry.spent, nullifier, true);

    // Verify pool has enough balance
    let total_output = output_amount + remainder_amount;
    assert!(balance::value(&pool.sui_balance) >= total_output, E_INSUFFICIENT_BALANCE);

    // Send output to stealth address
    if (output_amount > 0) {
        transfer::public_transfer(
            coin::from_balance(balance::split(&mut pool.sui_balance, output_amount), ctx),
            output_stealth,
        );
    };

    // Send remainder to stealth address (if any)
    if (remainder_amount > 0) {
        transfer::public_transfer(
            coin::from_balance(balance::split(&mut pool.sui_balance, remainder_amount), ctx),
            remainder_stealth,
        );
    };

    // Hash nullifier for event (don't reveal raw nullifier in events)
    let nullifier_hash = sui::hash::blake2b256(&nullifier);

    // Emit event
    event::emit(SwapExecutedEvent {
        nullifier_hash,
        output_stealth,
        remainder_stealth,
        output_amount,
        remainder_amount,
    });

    // Cleanup intent
    let SwapIntent { id, encrypted_details: _, token_in: _, token_out: _, deadline: _ } = intent;
    object::delete(id);
}

/// TEE marks a deposit as consumed after swap (optional cleanup)
/// This removes the deposit object from the blockchain
entry fun consume_deposit(pool: &LiquidityPool, deposit: Deposit, ctx: &TxContext) {
    // Only TEE can consume
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_TEE);

    let deposit_id = object::id(&deposit);

    // Emit event
    event::emit(DepositConsumedEvent {
        deposit_id,
    });

    // Destroy deposit
    let Deposit { id, encrypted_data: _, token_type: _, amount: _ } = deposit;
    object::delete(id);
}

/// TEE cancels an expired intent
entry fun cancel_expired_intent(pool: &LiquidityPool, intent: SwapIntent, ctx: &TxContext) {
    // Only TEE can cancel
    assert!(tx_context::sender(ctx) == pool.tee_authority, E_NOT_TEE);

    // Verify deadline passed
    assert!(tx_context::epoch_timestamp_ms(ctx) > intent.deadline, E_DEADLINE_PASSED);

    // Cleanup intent
    let SwapIntent { id, encrypted_details: _, token_in: _, token_out: _, deadline: _ } = intent;
    object::delete(id);
}

// ============ VIEW FUNCTIONS ============

/// Get deposit encrypted data (for TEE scanning)
public fun deposit_encrypted_data(deposit: &Deposit): vector<u8> {
    deposit.encrypted_data
}

/// Get deposit token type
public fun deposit_token_type(deposit: &Deposit): vector<u8> {
    deposit.token_type
}

/// Get deposit amount
public fun deposit_amount(deposit: &Deposit): u64 {
    deposit.amount
}

/// Get intent encrypted details (for TEE processing)
public fun intent_encrypted_details(intent: &SwapIntent): vector<u8> {
    intent.encrypted_details
}

/// Get intent deadline
public fun intent_deadline(intent: &SwapIntent): u64 {
    intent.deadline
}

/// Check if nullifier is spent
public fun is_nullifier_spent(registry: &NullifierRegistry, nullifier: vector<u8>): bool {
    table::contains(&registry.spent, nullifier)
}

/// Get pool SUI balance
public fun pool_sui_balance(pool: &LiquidityPool): u64 {
    balance::value(&pool.sui_balance)
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

/// Add liquidity (for testing/bootstrap)
entry fun add_liquidity_sui(pool: &mut LiquidityPool, payment: Coin<SUI>) {
    balance::join(&mut pool.sui_balance, coin::into_balance(payment));
}

// ============ TEST HELPERS ============

#[test_only]
public fun init_for_testing(ctx: &mut TxContext) {
    init(ctx);
}
