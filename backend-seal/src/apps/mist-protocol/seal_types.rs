// SEAL Types for Mist Protocol
// Simplified from seal-example for real-time decryption

use fastcrypto::encoding::{Encoding, Hex};
use fastcrypto::serde_helpers::ToFromByteArray;
use seal_sdk::IBEPublicKey;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use sui_sdk_types::ObjectId as ObjectID;

/// Custom deserializer for hex string to ObjectID
fn deserialize_object_id<'de, D>(deserializer: D) -> Result<ObjectID, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    ObjectID::from_str(&s).map_err(serde::de::Error::custom)
}

/// Custom deserializer for Vec of hex strings to Vec<ObjectID>
fn deserialize_object_ids<'de, D>(deserializer: D) -> Result<Vec<ObjectID>, D::Error>
where
    D: Deserializer<'de>,
{
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| ObjectID::from_str(&s).map_err(serde::de::Error::custom))
        .collect()
}

/// Custom deserializer for Vec of hex strings to Vec<IBEPublicKey>
fn deserialize_ibe_public_keys<'de, D>(deserializer: D) -> Result<Vec<IBEPublicKey>, D::Error>
where
    D: Deserializer<'de>,
{
    let pk_hexs: Vec<String> = Vec::deserialize(deserializer)?;
    pk_hexs
        .into_iter()
        .map(|pk_hex| {
            let pk_bytes = Hex::decode(&pk_hex).map_err(serde::de::Error::custom)?;
            let pk = IBEPublicKey::from_byte_array(
                &pk_bytes
                    .try_into()
                    .map_err(|_| serde::de::Error::custom("Invalid public key length"))?,
            )
            .map_err(serde::de::Error::custom)?;
            Ok(pk)
        })
        .collect()
}

/// Configuration for SEAL key servers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SealConfigRaw")]
pub struct SealConfig {
    pub key_servers: Vec<ObjectID>,
    pub public_keys: Vec<IBEPublicKey>,
    pub package_id: ObjectID,
    pub intent_queue_id: ObjectID,
    pub server_pk_map: HashMap<ObjectID, IBEPublicKey>,
}

#[derive(Debug, Deserialize)]
struct SealConfigRaw {
    #[serde(deserialize_with = "deserialize_object_ids")]
    key_servers: Vec<ObjectID>,
    #[serde(deserialize_with = "deserialize_ibe_public_keys")]
    public_keys: Vec<IBEPublicKey>,
    #[serde(deserialize_with = "deserialize_object_id")]
    package_id: ObjectID,
    #[serde(deserialize_with = "deserialize_object_id")]
    intent_queue_id: ObjectID,
}

impl TryFrom<SealConfigRaw> for SealConfig {
    type Error = String;

    fn try_from(raw: SealConfigRaw) -> Result<Self, Self::Error> {
        if raw.key_servers.len() != raw.public_keys.len() {
            return Err(format!(
                "key_servers and public_keys length mismatch: {} vs {}",
                raw.key_servers.len(),
                raw.public_keys.len()
            ));
        }

        let server_pk_map: HashMap<ObjectID, IBEPublicKey> = raw
            .key_servers
            .iter()
            .zip(raw.public_keys.iter())
            .map(|(id, pk)| (*id, *pk))
            .collect();

        Ok(SealConfig {
            key_servers: raw.key_servers,
            public_keys: raw.public_keys,
            package_id: raw.package_id,
            intent_queue_id: raw.intent_queue_id,
            server_pk_map,
        })
    }
}
