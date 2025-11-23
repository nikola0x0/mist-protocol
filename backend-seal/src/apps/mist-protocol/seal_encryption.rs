//! SEAL Encryption - Encrypts data using SEAL threshold encryption

use anyhow::Result;
use seal_sdk::seal_encrypt;
use sui_sdk_types::ObjectId;
use std::str::FromStr;

/// Encrypt amount with SEAL for output ticket
pub fn encrypt_amount(amount: u64, vault_id: &str) -> Result<Vec<u8>> {
    use crypto::{EncryptionInput, IBEPublicKeys};

    // Create encryption ID (vault namespace + random suffix)
    let vault_bytes = hex::decode(vault_id.trim_start_matches("0x"))?;
    let mut encryption_id = vault_bytes.clone();
    encryption_id.extend_from_slice(b"_output_");
    encryption_id.extend_from_slice(&rand::random::<[u8; 8]>());

    // Convert amount to bytes
    let amount_str = amount.to_string();
    let plaintext = amount_str.as_bytes().to_vec();

    // Get package ID and key servers from config
    let package_id = ObjectId::from_str(&super::SEAL_CONFIG.package_id.to_string())?;
    let key_servers = super::SEAL_CONFIG.key_servers.clone();
    let public_keys: Vec<_> = super::SEAL_CONFIG.server_pk_map.values().cloned().collect();

    // Encrypt using SEAL
    let (encrypted_obj, _symmetric_key) = seal_encrypt(
        package_id,
        encryption_id,
        key_servers,
        &IBEPublicKeys::BonehFranklinBLS12381(public_keys),
        2, // threshold = 2
        EncryptionInput::Aes256Gcm { data: plaintext, aad: None },
    )
    .map_err(|e| anyhow::anyhow!("SEAL encryption failed: {:?}", e))?;

    // Serialize to bytes
    Ok(bcs::to_bytes(&encrypted_obj)?)
}
