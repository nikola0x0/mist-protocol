//! Sui JSON-RPC client for querying objects and submitting transactions
//! This replaces the sui-sdk SuiClient with direct HTTP calls

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use sui_sdk_types::Address;

/// Sui JSON-RPC client
pub struct SuiRpcClient {
    client: Client,
    rpc_url: String,
}

/// Object reference: (object_id, version, digest)
pub type ObjectRef = (Address, u64, String);

/// Owner type for objects
/// Sui RPC returns owner in different formats:
/// - "Immutable" (string)
/// - { "AddressOwner": "0x..." }
/// - { "ObjectOwner": "0x..." }
/// - { "Shared": { "initial_shared_version": 1 } }
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum Owner {
    /// Shared object with initial version
    SharedObject {
        #[serde(rename = "Shared")]
        shared: SharedOwner,
    },
    /// Owned by an address
    Address {
        #[serde(rename = "AddressOwner")]
        address_owner: String,
    },
    /// Owned by another object
    Object {
        #[serde(rename = "ObjectOwner")]
        object_owner: String,
    },
    /// Immutable object (returned as string "Immutable")
    Immutable(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct SharedOwner {
    pub initial_shared_version: u64,
}

impl SuiRpcClient {
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: Client::new(),
            rpc_url,
        }
    }

    /// Make a JSON-RPC call
    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = response_json.get("error") {
            return Err(anyhow!("RPC error: {}", error));
        }

        let result = response_json
            .get("result")
            .ok_or_else(|| anyhow!("Missing result in RPC response"))?;

        serde_json::from_value(result.clone()).context("Failed to deserialize RPC result")
    }

    /// Get reference gas price
    #[allow(dead_code)]
    pub async fn get_reference_gas_price(&self) -> Result<u64> {
        let price: String = self.rpc_call("suix_getReferenceGasPrice", json!([])).await?;
        price.parse().context("Failed to parse gas price")
    }

    /// Get object with options
    pub async fn get_object(&self, object_id: &Address) -> Result<ObjectResponse> {
        let options = json!({
            "showContent": true,
            "showOwner": true,
            "showType": true
        });

        self.rpc_call(
            "sui_getObject",
            json!([object_id.to_string(), options]),
        )
        .await
    }

    /// Get object ref (id, version, digest)
    pub async fn get_object_ref(&self, object_id: &Address) -> Result<ObjectRef> {
        let obj = self.get_object(object_id).await?;
        let data = obj.data.ok_or_else(|| anyhow!("Object not found: {}", object_id))?;

        // For shared objects, use initial_shared_version
        let version = if let Some(Owner::SharedObject { shared }) = &data.owner {
            shared.initial_shared_version
        } else {
            data.version.parse().context("Failed to parse version")?
        };

        Ok((
            *object_id,
            version,
            data.digest,
        ))
    }

    /// Get initial shared version for a shared object
    #[allow(dead_code)]
    pub async fn get_initial_shared_version(&self, object_id: &Address) -> Result<u64> {
        let obj = self.get_object(object_id).await?;
        let data = obj.data.ok_or_else(|| anyhow!("Object not found: {}", object_id))?;

        match data.owner {
            Some(Owner::SharedObject { shared }) => Ok(shared.initial_shared_version),
            _ => Err(anyhow!("Object {} is not a shared object", object_id)),
        }
    }

    /// Get object type
    pub async fn get_object_type(&self, object_id: &Address) -> Result<String> {
        let obj = self.get_object(object_id).await?;
        let data = obj.data.ok_or_else(|| anyhow!("Object not found: {}", object_id))?;
        data.type_.ok_or_else(|| anyhow!("Object missing type info"))
    }

    /// Get dynamic field object
    pub async fn get_dynamic_field_object(
        &self,
        parent_id: &Address,
        name_type: &str,
        name_value: &str,
    ) -> Result<ObjectResponse> {
        let name = json!({
            "type": name_type,
            "value": name_value
        });

        self.rpc_call(
            "suix_getDynamicFieldObject",
            json!([parent_id.to_string(), name]),
        )
        .await
    }

    /// Extract field value from Move object content
    #[allow(dead_code)]
    pub fn extract_field_value(content: &Value, field_name: &str) -> Option<Value> {
        content
            .get("fields")?
            .get(field_name)
            .cloned()
    }

    /// Extract table ID from registry content
    pub fn extract_table_id(content: &Value) -> Result<Address> {
        let fields = content
            .get("fields")
            .ok_or_else(|| anyhow!("Missing fields in content"))?;

        let xid_to_account = fields
            .get("xid_to_account")
            .ok_or_else(|| anyhow!("Missing xid_to_account field"))?;

        let table_fields = xid_to_account
            .get("fields")
            .ok_or_else(|| anyhow!("Missing table fields"))?;

        let id_field = table_fields
            .get("id")
            .ok_or_else(|| anyhow!("Missing id field"))?;

        let id_str = id_field
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Failed to extract table id"))?;

        id_str.parse().context("Failed to parse table id as Address")
    }

    /// Extract account ID from dynamic field value
    pub fn extract_account_id(content: &Value) -> Result<Address> {
        let fields = content
            .get("fields")
            .ok_or_else(|| anyhow!("Missing fields in content"))?;

        let value = fields
            .get("value")
            .ok_or_else(|| anyhow!("Missing value field"))?;

        let id_str = value
            .as_str()
            .ok_or_else(|| anyhow!("Value is not a string"))?;

        id_str.parse().context("Failed to parse account id as Address")
    }
}

/// Response from sui_getObject
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ObjectResponse {
    pub data: Option<ObjectData>,
    pub error: Option<Value>,
}

/// Object data from RPC response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ObjectData {
    pub object_id: String,
    pub version: String,
    pub digest: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub owner: Option<Owner>,
    pub content: Option<Value>,
}
