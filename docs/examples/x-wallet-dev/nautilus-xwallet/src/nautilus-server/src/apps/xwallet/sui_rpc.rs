// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Sui RPC Client for NFT name lookup
//!
//! This module provides functionality to query the Sui blockchain
//! to resolve NFT names to object IDs.

use tracing::info;

/// Errors that can occur during Sui RPC operations
#[derive(Debug)]
pub enum SuiRpcError {
    /// HTTP request failed
    RequestFailed(String),
    /// Failed to parse RPC response
    ParseError(String),
    /// Account not found in registry
    AccountNotFound,
    /// RPC returned an error
    RpcError(String),
}

impl std::fmt::Display for SuiRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuiRpcError::RequestFailed(e) => write!(f, "RPC request failed: {}", e),
            SuiRpcError::ParseError(e) => write!(f, "Failed to parse RPC response: {}", e),
            SuiRpcError::AccountNotFound => write!(f, "Account not found in registry"),
            SuiRpcError::RpcError(e) => write!(f, "RPC error: {}", e),
        }
    }
}

impl std::error::Error for SuiRpcError {}

/// NFT metadata from Sui Display standard
#[derive(Debug, Clone)]
pub struct NftMetadata {
    pub name: Option<String>,
    pub image_url: Option<String>,
    pub nft_type: String,
}

/// A matched NFT with its metadata
#[derive(Debug, Clone)]
pub struct NftMatch {
    pub object_id: String,
    pub metadata: NftMetadata,
}

/// Result of NFT lookup by name
#[derive(Debug)]
pub enum NftLookupResult {
    /// Exactly one NFT found with the given name
    Found(NftMatch),
    /// No NFT found with the given name
    NotFound,
    /// Multiple NFTs found with the same name
    Multiple(Vec<NftMatch>),
}

/// Sui RPC client for querying blockchain state
pub struct SuiRpcClient {
    client: reqwest::Client,
    rpc_url: String,
}

impl SuiRpcClient {
    /// Create a new Sui RPC client
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url: rpc_url.to_string(),
        }
    }

    /// Make a JSON-RPC call to Sui
    async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, SuiRpcError> {
        let request = serde_json::json!({
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
            .map_err(|e| SuiRpcError::RequestFailed(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SuiRpcError::ParseError(e.to_string()))?;

        // Check for RPC error
        if let Some(error) = json.get("error") {
            return Err(SuiRpcError::RpcError(error.to_string()));
        }

        Ok(json)
    }

    /// Get XWalletAccount object ID from Registry by XID
    ///
    /// The registry has a `xid_to_account` Table field where:
    /// - Key: XID (Twitter user ID as string)
    /// - Value: XWalletAccount object ID
    pub async fn get_account_id_by_xid(
        &self,
        registry_id: &str,
        xid: &str,
    ) -> Result<Option<String>, SuiRpcError> {
        info!("Looking up account for XID: {} in registry: {}", xid, registry_id);

        // First, get the registry object to find the xid_to_account table ID
        let params = serde_json::json!([
            registry_id,
            {
                "showContent": true
            }
        ]);

        let response = self.rpc_call("sui_getObject", params).await?;

        // Extract the xid_to_account table ID from registry
        let table_id = response
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("content"))
            .and_then(|c| c.get("fields"))
            .and_then(|f| f.get("xid_to_account"))
            .and_then(|t| t.get("fields"))
            .and_then(|f| f.get("id"))
            .and_then(|id| id.get("id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| SuiRpcError::ParseError("Failed to extract xid_to_account table ID".to_string()))?;

        info!("Found xid_to_account table ID: {}", table_id);

        // Now query the table for the XID
        let params = serde_json::json!([
            table_id,
            {
                "type": "0x1::string::String",
                "value": xid
            }
        ]);

        let response = self.rpc_call("suix_getDynamicFieldObject", params).await?;

        // Check if result exists
        let result = response.get("result");
        if result.is_none() {
            return Ok(None);
        }

        let result = result.unwrap();

        // Check for error in result (field not found)
        if result.get("error").is_some() {
            return Ok(None);
        }

        // Extract the account object ID from the dynamic field value
        // The structure is: result.data.content.fields.value (which is the account ID)
        let account_id = result
            .get("data")
            .and_then(|d| d.get("content"))
            .and_then(|c| c.get("fields"))
            .and_then(|f| f.get("value"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        info!("Found account ID: {:?}", account_id);
        Ok(account_id)
    }

    /// Get list of NFT object IDs from XWalletAccount.nfts (ObjectBag)
    ///
    /// The account has an `nfts` field which is an ObjectBag containing NFTs
    pub async fn get_account_nft_ids(
        &self,
        account_id: &str,
    ) -> Result<Vec<String>, SuiRpcError> {
        info!("Getting NFT IDs for account: {}", account_id);

        // First, get the account object to find the nfts ObjectBag ID
        let params = serde_json::json!([
            account_id,
            {
                "showContent": true,
                "showType": true
            }
        ]);

        let response = self.rpc_call("sui_getObject", params).await?;

        // Extract the nfts ObjectBag ID
        let nfts_bag_id = response
            .get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("content"))
            .and_then(|c| c.get("fields"))
            .and_then(|f| f.get("nfts"))
            .and_then(|n| n.get("fields"))
            .and_then(|f| f.get("id"))
            .and_then(|id| id.get("id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| SuiRpcError::ParseError("Failed to extract nfts ObjectBag ID".to_string()))?;

        info!("Found nfts ObjectBag ID: {}", nfts_bag_id);

        // List all dynamic fields in the ObjectBag to get NFT IDs
        let mut nft_ids = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let params = if let Some(ref c) = cursor {
                serde_json::json!([nfts_bag_id, c, 50])
            } else {
                serde_json::json!([nfts_bag_id, null, 50])
            };

            let response = self.rpc_call("suix_getDynamicFields", params).await?;

            let result = response.get("result").ok_or_else(|| {
                SuiRpcError::ParseError("Missing result in getDynamicFields response".to_string())
            })?;

            // Extract NFT IDs from dynamic fields
            if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
                for field in data {
                    // The objectId is the dynamic field object, we need the actual NFT ID
                    // which is stored in the field's name/value
                    if let Some(object_id) = field.get("objectId").and_then(|id| id.as_str()) {
                        nft_ids.push(object_id.to_string());
                    }
                }
            }

            // Check for next page
            let has_next = result
                .get("hasNextPage")
                .and_then(|h| h.as_bool())
                .unwrap_or(false);

            if !has_next {
                break;
            }

            cursor = result
                .get("nextCursor")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string());
        }

        info!("Found {} NFTs in account", nft_ids.len());
        Ok(nft_ids)
    }

    /// Get NFT metadata (name, image_url, type) using Display standard
    pub async fn get_nft_metadata(&self, nft_id: &str) -> Result<NftMetadata, SuiRpcError> {
        let params = serde_json::json!([
            nft_id,
            {
                "showContent": true,
                "showDisplay": true,
                "showType": true
            }
        ]);

        let response = self.rpc_call("sui_getObject", params).await?;

        let result = response.get("result").ok_or_else(|| {
            SuiRpcError::ParseError("Missing result in getObject response".to_string())
        })?;

        let data = result.get("data").ok_or_else(|| {
            SuiRpcError::ParseError("Missing data in getObject response".to_string())
        })?;

        // Get NFT type
        let nft_type = data
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Try to get display data (Sui Display standard)
        let display = data.get("display").and_then(|d| d.get("data"));

        let name = display
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let image_url = display
            .and_then(|d| d.get("image_url"))
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        // Fallback: try to get name from content.fields if display is not available
        let name = name.or_else(|| {
            data.get("content")
                .and_then(|c| c.get("fields"))
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        });

        Ok(NftMetadata {
            name,
            image_url,
            nft_type,
        })
    }

    /// Find NFT by name in user's account
    ///
    /// This function:
    /// 1. Gets the user's XWalletAccount from the registry by XID
    /// 2. Lists all NFTs in the account's ObjectBag
    /// 3. Queries metadata for each NFT
    /// 4. Filters by name and returns the result
    pub async fn find_nft_by_name(
        &self,
        registry_id: &str,
        xid: &str,
        nft_name: &str,
    ) -> Result<NftLookupResult, SuiRpcError> {
        info!("Finding NFT '{}' for XID: {}", nft_name, xid);

        // 1. Get account ID from registry
        let account_id = self
            .get_account_id_by_xid(registry_id, xid)
            .await?
            .ok_or(SuiRpcError::AccountNotFound)?;

        // 2. Get all NFT IDs from account
        let nft_ids = self.get_account_nft_ids(&account_id).await?;

        if nft_ids.is_empty() {
            info!("No NFTs found in account");
            return Ok(NftLookupResult::NotFound);
        }

        // 3. Query metadata for each NFT and filter by name
        // Note: Sequential iteration for simplicity. Can optimize with parallel queries later.
        let nft_name_lower = nft_name.to_lowercase();
        let mut matches = Vec::new();

        for nft_id in &nft_ids {
            if let Ok(metadata) = self.get_nft_metadata(nft_id).await {
                if let Some(ref name) = metadata.name {
                    if name.to_lowercase() == nft_name_lower {
                        matches.push(NftMatch {
                            object_id: nft_id.clone(),
                            metadata,
                        });

                        // Early termination optimization: if we found more than one,
                        // we already know we need to return Multiple
                        if matches.len() > 1 {
                            break;
                        }
                    }
                }
            }
        }

        // 5. Return result based on match count
        match matches.len() {
            0 => {
                info!("No NFT found with name '{}'", nft_name);
                Ok(NftLookupResult::NotFound)
            }
            1 => {
                let nft = matches.remove(0);
                info!("Found NFT '{}' with ID: {}", nft_name, nft.object_id);
                Ok(NftLookupResult::Found(nft))
            }
            n => {
                info!("Found {} NFTs with name '{}'", n, nft_name);
                Ok(NftLookupResult::Multiple(matches))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sui_rpc_client_creation() {
        let client = SuiRpcClient::new("https://fullnode.mainnet.sui.io:443");
        assert_eq!(client.rpc_url, "https://fullnode.mainnet.sui.io:443");
    }

    #[test]
    fn test_nft_lookup_result_variants() {
        // Test NotFound
        let result = NftLookupResult::NotFound;
        assert!(matches!(result, NftLookupResult::NotFound));

        // Test Found
        let nft = NftMatch {
            object_id: "0x123".to_string(),
            metadata: NftMetadata {
                name: Some("Test NFT".to_string()),
                image_url: None,
                nft_type: "test::TestNFT".to_string(),
            },
        };
        let result = NftLookupResult::Found(nft);
        assert!(matches!(result, NftLookupResult::Found(_)));

        // Test Multiple
        let matches = vec![
            NftMatch {
                object_id: "0x123".to_string(),
                metadata: NftMetadata {
                    name: Some("Test NFT".to_string()),
                    image_url: None,
                    nft_type: "test::TestNFT".to_string(),
                },
            },
            NftMatch {
                object_id: "0x456".to_string(),
                metadata: NftMetadata {
                    name: Some("Test NFT".to_string()),
                    image_url: None,
                    nft_type: "fake::FakeNFT".to_string(),
                },
            },
        ];
        let result = NftLookupResult::Multiple(matches);
        assert!(matches!(result, NftLookupResult::Multiple(_)));
    }
}
