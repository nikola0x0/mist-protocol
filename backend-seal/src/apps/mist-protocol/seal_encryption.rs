//! SEAL Encryption - Encrypts data using SEAL threshold encryption

use anyhow::Result;
use seal_sdk::seal_encrypt;
use sui_sdk_types::ObjectId;
use std::str::FromStr;

/// Encrypt amount with SEAL for output ticket
pub fn encrypt_amount(amount: u64, vault_id: &str) -> Result<Vec<u8>> {
    use crypto::{EncryptionInput, IBEPublicKeys};

    // Create encryption ID (vault namespace + random nonce, matching frontend pattern)
    let vault_bytes = hex::decode(vault_id.trim_start_matches("0x"))?;
    let nonce = rand::random::<[u8; 5]>(); // Use 5 bytes like frontend
    let mut encryption_id = vault_bytes.clone();
    encryption_id.extend_from_slice(&nonce);

    let enc_id_hex = hex::encode(&encryption_id);
    println!("   🔐 SEAL Encryption ID (full): 0x{}", enc_id_hex);
    println!("   📊 Encryption ID length: {} bytes (vault: 32, nonce: 5)", encryption_id.len());

    // Convert amount to bytes (as string, like frontend does)
    let amount_str = amount.to_string();
    let plaintext = amount_str.as_bytes().to_vec();
    println!("   💰 Encrypting amount: {} (as string: '{}')", amount, amount_str);

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
    let encrypted_bytes = bcs::to_bytes(&encrypted_obj)?;
    println!("   ✅ Encrypted successfully! Size: {} bytes", encrypted_bytes.len());
    println!("");

    Ok(encrypted_bytes)
}
