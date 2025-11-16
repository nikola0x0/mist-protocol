// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Question Bank - SEAL Namespace with Creator-Only Access
 *
 * Based on the allowlist pattern but simplified:
 * - No list management needed (single creator)
 * - Just namespace (UID) for SEAL encryption
 * - Creator-only access control
 *
 * Key format: [pkg id]::[question_bank_id][nonce]
 */

module walrus::question_bank;

use std::string::String;
use walrus::utils::is_prefix;

/// Error codes
const ENoAccess: u64 = 0;

/// Question Bank - namespace for creator's questions
public struct QuestionBank has key {
    id: UID,
    name: String,
    creator: address, // Only this address can decrypt
}

/// Admin capability for publishing blob IDs
public struct Cap has key {
    id: UID,
    bank_id: ID,
}

//////////////////////////////////////////
/// Creation

/// Create a Question Bank with an admin cap
/// Key format: [pkg id]::[bank_id][nonce]
public fun create_question_bank(name: String, ctx: &mut TxContext): Cap {
    let bank = QuestionBank {
        id: object::new(ctx),
        name,
        creator: ctx.sender(),
    };
    let cap = Cap {
        id: object::new(ctx),
        bank_id: object::id(&bank),
    };
    transfer::share_object(bank);
    cap
}

/// Convenience entry function
entry fun create_question_bank_entry(name: String, ctx: &mut TxContext) {
    transfer::transfer(create_question_bank(name, ctx), ctx.sender());
}

//////////////////////////////////////////
/// Namespace & Access Control

/// Get the namespace for SEAL encryption
public fun namespace(bank: &QuestionBank): vector<u8> {
    bank.id.to_bytes()
}

/// Internal approval logic - only creator can access
fun approve_internal(caller: address, id: vector<u8>, bank: &QuestionBank): bool {
    // Check if the id has the right prefix (namespace match)
    let namespace_bytes = namespace(bank);
    if (!is_prefix(namespace_bytes, id)) {
        return false
    };

    // Check if caller is the creator (no list needed - just owner check!)
    caller == bank.creator
}

/// SEAL approval - called by SEAL SDK
entry fun seal_approve(id: vector<u8>, bank: &QuestionBank, ctx: &TxContext) {
    assert!(approve_internal(ctx.sender(), id, bank), ENoAccess);
}

//////////////////////////////////////////
// NOTE: Blob IDs are NOT stored on-chain
// All blob ID tracking happens in the backend database
// The contract only provides SEAL namespace and access control

//////////////////////////////////////////
/// Getters

public fun get_creator(bank: &QuestionBank): address {
    bank.creator
}

public fun get_name(bank: &QuestionBank): String {
    bank.name
}

//////////////////////////////////////////
/// Test helpers

#[test_only]
public fun new_bank_for_testing(ctx: &mut TxContext): QuestionBank {
    QuestionBank {
        id: object::new(ctx),
        name: b"test".to_string(),
        creator: ctx.sender(),
    }
}

#[test_only]
public fun new_cap_for_testing(ctx: &mut TxContext, bank: &QuestionBank): Cap {
    Cap {
        id: object::new(ctx),
        bank_id: object::id(bank),
    }
}

#[test_only]
public fun destroy_for_testing(bank: QuestionBank, cap: Cap) {
    let QuestionBank { id, .. } = bank;
    object::delete(id);
    let Cap { id, .. } = cap;
    object::delete(id);
}
