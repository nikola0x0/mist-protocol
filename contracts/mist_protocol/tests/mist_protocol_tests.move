#[test_only]
module mist_protocol::mist_protocol_tests;

use sui::test_scenario::{Self as ts, Scenario};
use sui::coin::{Self, Coin};
use sui::sui::SUI;
use mist_protocol::mist_protocol::{
    Self,
    LiquidityPool,
    NullifierRegistry,
    Deposit,
    SwapIntent,
    AdminCap,
};

// ============ TEST CONSTANTS ============

const ADMIN: address = @0xAD;
const USER1: address = @0x1;
const USER2: address = @0x2;
const TEE: address = @0x9bf64712c379154caeca62619795dbc0c839f3299518450796598a68407c2ff0;
const STEALTH1: address = @0x51EA1;
const STEALTH2: address = @0x51EA2;

// ============ HELPER FUNCTIONS ============

fun setup_test(): Scenario {
    let mut scenario = ts::begin(ADMIN);

    // Initialize the protocol (creates pool, registry, admin cap)
    ts::next_tx(&mut scenario, ADMIN);
    {
        mist_protocol::init_for_testing(ts::ctx(&mut scenario));
    };

    scenario
}

fun mint_sui(scenario: &mut Scenario, amount: u64): Coin<SUI> {
    coin::mint_for_testing<SUI>(amount, ts::ctx(scenario))
}

// ============ DEPOSIT TESTS ============

#[test]
fun test_deposit_sui_success() {
    let mut scenario = setup_test();

    // User deposits SUI
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 1_000_000_000); // 1 SUI
        let encrypted_data = b"encrypted_nullifier_data";

        mist_protocol::deposit_sui(
            &mut pool,
            payment,
            encrypted_data,
            ts::ctx(&mut scenario),
        );

        // Verify pool balance increased
        assert!(mist_protocol::pool_sui_balance(&pool) == 1_000_000_000, 0);

        ts::return_shared(pool);
    };

    // Verify deposit object was created
    ts::next_tx(&mut scenario, USER1);
    {
        let deposit = ts::take_shared<Deposit>(&scenario);
        assert!(mist_protocol::deposit_amount(&deposit) == 1_000_000_000, 1);
        assert!(mist_protocol::deposit_token_type(&deposit) == b"SUI", 2);
        ts::return_shared(deposit);
    };

    ts::end(scenario);
}

#[test]
fun test_multiple_deposits() {
    let mut scenario = setup_test();

    // User1 deposits
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 500_000_000);
        mist_protocol::deposit_sui(&mut pool, payment, b"enc1", ts::ctx(&mut scenario));
        ts::return_shared(pool);
    };

    // User2 deposits
    ts::next_tx(&mut scenario, USER2);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 300_000_000);
        mist_protocol::deposit_sui(&mut pool, payment, b"enc2", ts::ctx(&mut scenario));

        // Total should be 800M
        assert!(mist_protocol::pool_sui_balance(&pool) == 800_000_000, 0);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = mist_protocol::E_PAUSED)]
fun test_deposit_when_paused() {
    let mut scenario = setup_test();

    // Admin pauses the pool
    ts::next_tx(&mut scenario, ADMIN);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let admin_cap = ts::take_from_sender<AdminCap>(&scenario);

        mist_protocol::set_pause(&mut pool, &admin_cap, true);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(pool);
    };

    // User tries to deposit - should fail
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 1_000_000_000);

        // This should abort with E_PAUSED
        mist_protocol::deposit_sui(&mut pool, payment, b"enc", ts::ctx(&mut scenario));

        ts::return_shared(pool);
    };

    ts::end(scenario);
}

// ============ SWAP INTENT TESTS ============

#[test]
fun test_create_swap_intent_success() {
    let mut scenario = setup_test();

    // User creates swap intent
    ts::next_tx(&mut scenario, USER1);
    {
        let encrypted_details = b"encrypted_swap_details";
        let deadline = 9999999999999u64; // Far future

        mist_protocol::create_swap_intent(
            encrypted_details,
            b"SUI",
            b"SUI",
            deadline,
            ts::ctx(&mut scenario),
        );
    };

    // Verify intent was created
    ts::next_tx(&mut scenario, USER1);
    {
        let intent = ts::take_shared<SwapIntent>(&scenario);
        assert!(mist_protocol::intent_deadline(&intent) == 9999999999999u64, 0);
        ts::return_shared(intent);
    };

    ts::end(scenario);
}

// ============ EXECUTE SWAP TESTS ============

#[test]
fun test_execute_swap_success() {
    let mut scenario = setup_test();

    // First add liquidity to the pool
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 10_000_000_000); // 10 SUI
        mist_protocol::add_liquidity_sui(&mut pool, payment);
        ts::return_shared(pool);
    };

    // Create swap intent
    ts::next_tx(&mut scenario, USER1);
    {
        mist_protocol::create_swap_intent(
            b"encrypted_details",
            b"SUI",
            b"SUI",
            9999999999999u64,
            ts::ctx(&mut scenario),
        );
    };

    // TEE executes swap
    ts::next_tx(&mut scenario, TEE);
    {
        let mut registry = ts::take_shared<NullifierRegistry>(&scenario);
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let intent = ts::take_shared<SwapIntent>(&scenario);

        let nullifier = b"unique_nullifier_123";

        mist_protocol::execute_swap(
            &mut registry,
            &mut pool,
            intent,
            nullifier,
            800_000_000,  // output amount
            STEALTH1,
            200_000_000,  // remainder
            STEALTH2,
            ts::ctx(&mut scenario),
        );

        // Verify nullifier is now spent
        assert!(mist_protocol::is_nullifier_spent(&registry, nullifier), 0);

        // Verify pool balance decreased
        assert!(mist_protocol::pool_sui_balance(&pool) == 9_000_000_000, 1);

        ts::return_shared(registry);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = mist_protocol::E_NOT_TEE)]
fun test_execute_swap_non_tee_fails() {
    let mut scenario = setup_test();

    // Add liquidity
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 10_000_000_000);
        mist_protocol::add_liquidity_sui(&mut pool, payment);
        ts::return_shared(pool);
    };

    // Create swap intent
    ts::next_tx(&mut scenario, USER1);
    {
        mist_protocol::create_swap_intent(
            b"encrypted_details",
            b"SUI",
            b"SUI",
            9999999999999u64,
            ts::ctx(&mut scenario),
        );
    };

    // Non-TEE tries to execute - should fail
    ts::next_tx(&mut scenario, USER1); // Not TEE!
    {
        let mut registry = ts::take_shared<NullifierRegistry>(&scenario);
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let intent = ts::take_shared<SwapIntent>(&scenario);

        // This should abort with E_NOT_TEE
        mist_protocol::execute_swap(
            &mut registry,
            &mut pool,
            intent,
            b"nullifier",
            500_000_000,
            STEALTH1,
            500_000_000,
            STEALTH2,
            ts::ctx(&mut scenario),
        );

        ts::return_shared(registry);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = mist_protocol::E_NULLIFIER_SPENT)]
fun test_double_spend_prevention() {
    let mut scenario = setup_test();

    // Add liquidity
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 20_000_000_000); // 20 SUI
        mist_protocol::add_liquidity_sui(&mut pool, payment);
        ts::return_shared(pool);
    };

    // Create first swap intent
    ts::next_tx(&mut scenario, USER1);
    {
        mist_protocol::create_swap_intent(
            b"enc1", b"SUI", b"SUI", 9999999999999u64,
            ts::ctx(&mut scenario),
        );
    };

    // TEE executes first swap
    ts::next_tx(&mut scenario, TEE);
    {
        let mut registry = ts::take_shared<NullifierRegistry>(&scenario);
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let intent = ts::take_shared<SwapIntent>(&scenario);

        let nullifier = b"same_nullifier";

        mist_protocol::execute_swap(
            &mut registry, &mut pool, intent, nullifier,
            500_000_000, STEALTH1, 500_000_000, STEALTH2,
            ts::ctx(&mut scenario),
        );

        ts::return_shared(registry);
        ts::return_shared(pool);
    };

    // Create second swap intent
    ts::next_tx(&mut scenario, USER1);
    {
        mist_protocol::create_swap_intent(
            b"enc2", b"SUI", b"SUI", 9999999999999u64,
            ts::ctx(&mut scenario),
        );
    };

    // TEE tries to use same nullifier - should fail
    ts::next_tx(&mut scenario, TEE);
    {
        let mut registry = ts::take_shared<NullifierRegistry>(&scenario);
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let intent = ts::take_shared<SwapIntent>(&scenario);

        let nullifier = b"same_nullifier"; // SAME as before!

        // This should abort with E_NULLIFIER_SPENT
        mist_protocol::execute_swap(
            &mut registry, &mut pool, intent, nullifier,
            500_000_000, STEALTH1, 500_000_000, STEALTH2,
            ts::ctx(&mut scenario),
        );

        ts::return_shared(registry);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = mist_protocol::E_INSUFFICIENT_BALANCE)]
fun test_insufficient_balance() {
    let mut scenario = setup_test();

    // Add small amount of liquidity
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 100_000_000); // 0.1 SUI
        mist_protocol::add_liquidity_sui(&mut pool, payment);
        ts::return_shared(pool);
    };

    // Create swap intent
    ts::next_tx(&mut scenario, USER1);
    {
        mist_protocol::create_swap_intent(
            b"enc", b"SUI", b"SUI", 9999999999999u64,
            ts::ctx(&mut scenario),
        );
    };

    // TEE tries to execute swap for more than pool has
    ts::next_tx(&mut scenario, TEE);
    {
        let mut registry = ts::take_shared<NullifierRegistry>(&scenario);
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let intent = ts::take_shared<SwapIntent>(&scenario);

        // Request 10 SUI but pool only has 0.1 SUI
        mist_protocol::execute_swap(
            &mut registry, &mut pool, intent, b"nullifier",
            5_000_000_000, STEALTH1, // 5 SUI
            5_000_000_000, STEALTH2, // 5 SUI
            ts::ctx(&mut scenario),
        );

        ts::return_shared(registry);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

// ============ ADMIN TESTS ============

#[test]
fun test_update_tee_authority() {
    let mut scenario = setup_test();
    let new_tee: address = @0x123456;

    ts::next_tx(&mut scenario, ADMIN);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let admin_cap = ts::take_from_sender<AdminCap>(&scenario);

        mist_protocol::update_tee_authority(&mut pool, &admin_cap, new_tee);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

#[test]
fun test_pause_and_unpause() {
    let mut scenario = setup_test();

    // Pause
    ts::next_tx(&mut scenario, ADMIN);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let admin_cap = ts::take_from_sender<AdminCap>(&scenario);

        mist_protocol::set_pause(&mut pool, &admin_cap, true);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(pool);
    };

    // Unpause
    ts::next_tx(&mut scenario, ADMIN);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let admin_cap = ts::take_from_sender<AdminCap>(&scenario);

        mist_protocol::set_pause(&mut pool, &admin_cap, false);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(pool);
    };

    // Now deposit should work
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 1_000_000_000);

        mist_protocol::deposit_sui(&mut pool, payment, b"enc", ts::ctx(&mut scenario));

        assert!(mist_protocol::pool_sui_balance(&pool) == 1_000_000_000, 0);
        ts::return_shared(pool);
    };

    ts::end(scenario);
}

// ============ VIEW FUNCTION TESTS ============

#[test]
fun test_view_functions() {
    let mut scenario = setup_test();

    // Create deposit and test view functions
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 123_456_789);
        mist_protocol::deposit_sui(&mut pool, payment, b"test_encrypted_data", ts::ctx(&mut scenario));
        ts::return_shared(pool);
    };

    ts::next_tx(&mut scenario, USER1);
    {
        let deposit = ts::take_shared<Deposit>(&scenario);
        let pool = ts::take_shared<LiquidityPool>(&scenario);
        let registry = ts::take_shared<NullifierRegistry>(&scenario);

        // Test deposit view functions
        assert!(mist_protocol::deposit_amount(&deposit) == 123_456_789, 0);
        assert!(mist_protocol::deposit_token_type(&deposit) == b"SUI", 1);
        assert!(mist_protocol::deposit_encrypted_data(&deposit) == b"test_encrypted_data", 2);

        // Test pool view function
        assert!(mist_protocol::pool_sui_balance(&pool) == 123_456_789, 3);

        // Test nullifier check (should be false for unused)
        assert!(!mist_protocol::is_nullifier_spent(&registry, b"unused_nullifier"), 4);

        ts::return_shared(deposit);
        ts::return_shared(pool);
        ts::return_shared(registry);
    };

    ts::end(scenario);
}

// ============ CONSUME DEPOSIT TESTS ============

#[test]
fun test_consume_deposit() {
    let mut scenario = setup_test();

    // Create deposit
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 1_000_000_000);
        mist_protocol::deposit_sui(&mut pool, payment, b"enc", ts::ctx(&mut scenario));
        ts::return_shared(pool);
    };

    // TEE consumes deposit
    ts::next_tx(&mut scenario, TEE);
    {
        let pool = ts::take_shared<LiquidityPool>(&scenario);
        let deposit = ts::take_shared<Deposit>(&scenario);

        mist_protocol::consume_deposit(&pool, deposit, ts::ctx(&mut scenario));

        ts::return_shared(pool);
        // deposit is consumed, no need to return
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = mist_protocol::E_NOT_TEE)]
fun test_consume_deposit_non_tee_fails() {
    let mut scenario = setup_test();

    // Create deposit
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);
        let payment = mint_sui(&mut scenario, 1_000_000_000);
        mist_protocol::deposit_sui(&mut pool, payment, b"enc", ts::ctx(&mut scenario));
        ts::return_shared(pool);
    };

    // Non-TEE tries to consume - should fail
    ts::next_tx(&mut scenario, USER1);
    {
        let pool = ts::take_shared<LiquidityPool>(&scenario);
        let deposit = ts::take_shared<Deposit>(&scenario);

        // This should abort with E_NOT_TEE
        mist_protocol::consume_deposit(&pool, deposit, ts::ctx(&mut scenario));

        ts::return_shared(pool);
    };

    ts::end(scenario);
}

// ============ ADD LIQUIDITY TESTS ============

#[test]
fun test_add_liquidity() {
    let mut scenario = setup_test();

    // Anyone can add liquidity
    ts::next_tx(&mut scenario, USER1);
    {
        let mut pool = ts::take_shared<LiquidityPool>(&scenario);

        let payment1 = mint_sui(&mut scenario, 1_000_000_000);
        mist_protocol::add_liquidity_sui(&mut pool, payment1);
        assert!(mist_protocol::pool_sui_balance(&pool) == 1_000_000_000, 0);

        let payment2 = mint_sui(&mut scenario, 2_000_000_000);
        mist_protocol::add_liquidity_sui(&mut pool, payment2);
        assert!(mist_protocol::pool_sui_balance(&pool) == 3_000_000_000, 1);

        ts::return_shared(pool);
    };

    ts::end(scenario);
}
