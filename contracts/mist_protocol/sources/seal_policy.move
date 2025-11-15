// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Seal Policy for Mist Protocol
/// This module implements the seal_approve function that grants permission
/// for the Nautilus TEE to decrypt swap intents using Seal threshold encryption.

module mist_protocol::seal_policy {
    use enclave::enclave::Enclave;
    use mist_protocol::mist_protocol::MIST_PROTOCOL;
    use sui::hash::blake2b256;

    /// Error: Caller doesn't have permission to approve
    const ENoAccess: u64 = 0;

    /// Approve decryption for a given ID using Seal threshold encryption.
    /// This function verifies that the caller is the registered enclave
    /// by checking that the transaction sender matches the enclave's public key.
    ///
    /// Called by the TEE before requesting decryption from Seal servers.
    /// The _id parameter is the key ID for the encrypted data.
    entry fun seal_approve(_id: vector<u8>, enclave: &Enclave<MIST_PROTOCOL>, ctx: &TxContext) {
        // Verify that the transaction sender is the registered enclave
        // This ensures only the legitimate TEE can approve decryption
        assert!(ctx.sender().to_bytes() == pk_to_address(enclave.pk()), ENoAccess);
    }

    /// Convert enclave public key to address
    /// Assumes ed25519 flag for enclave's ephemeral key
    /// Derives address as blake2b_hash(flag || pk)
    fun pk_to_address(pk: &vector<u8>): vector<u8> {
        let mut arr = vector[0u8]; // ed25519 flag
        arr.append(*pk);
        let hash = blake2b256(&arr);
        hash
    }

    #[test]
    fun test_pk_to_address() {
        let eph_pk = x"5c38d3668c45ff891766ee99bd3522ae48d9771dc77e8a6ac9f0bde6c3a2ca48";
        let expected_bytes = x"29287d8584fb5b71b8d62e7224b867207d205fb61d42b7cce0deef95bf4e8202";
        assert!(pk_to_address(&eph_pk) == expected_bytes, ENoAccess);
    }
}
