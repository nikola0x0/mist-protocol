use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const XWALLET_MODULE: &str = "events";

#[derive(Clone)]
pub struct SuiClient {
    rpc_url: String,
    http: Client,
}

impl SuiClient {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            http: Client::new(),
        }
    }

    /// Fetch object data including display fields (name, image_url, etc.)
    pub async fn get_object(&self, object_id: &str) -> Result<Option<ObjectData>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "sui_getObject",
            "params": [
                object_id,
                {
                    "showType": true,
                    "showContent": true,
                    "showDisplay": true,
                }
            ],
        });

        let resp = self
            .http
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .context("failed to call sui_getObject")?;

        let rpc_resp: RpcResponse<ObjectResponse> = resp
            .json()
            .await
            .context("failed to parse sui_getObject response json")?;

        if let Some(err) = rpc_resp.error {
            return Err(anyhow!("Sui RPC error {}: {}", err.code, err.message));
        }

        Ok(rpc_resp.result.and_then(|r| r.data))
    }

    /// Fetch coin metadata (decimals, symbol, name, etc.)
    pub async fn get_coin_metadata(&self, coin_type: &str) -> Result<Option<CoinMetadata>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "suix_getCoinMetadata",
            "params": [coin_type],
        });

        let resp = self
            .http
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .context("failed to call suix_getCoinMetadata")?;

        let rpc_resp: RpcResponse<Option<CoinMetadata>> = resp
            .json()
            .await
            .context("failed to parse suix_getCoinMetadata response json")?;

        if let Some(err) = rpc_resp.error {
            return Err(anyhow!("Sui RPC error {}: {}", err.code, err.message));
        }

        Ok(rpc_resp.result.flatten())
    }

    pub async fn query_events(
        &self,
        package_id: &str,
        module: &str,
        cursor: Option<&str>,
        limit: u64,
    ) -> Result<EventPage> {
        let filter = json!({
            "MoveEventModule": {
                "package": package_id,
                "module": module,
            }
        });

        let cursor_value = cursor
            .and_then(EventId::from_cursor_str)
            .map(|id| json!(id))
            .unwrap_or(Value::Null);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "suix_queryEvents",
            "params": [filter, cursor_value, limit, false],
        });

        let resp = self
            .http
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .context("failed to call suix_queryEvents")?;

        let status = resp.status();
        let rpc_resp: RpcResponse<EventPage> = resp
            .json()
            .await
            .context("failed to parse suix_queryEvents response json")?;

        if let Some(err) = rpc_resp.error {
            return Err(anyhow!("Sui RPC error {}: {}", err.code, err.message));
        }

        rpc_resp
            .result
            .ok_or_else(|| anyhow!("empty Sui RPC response (status: {})", status))
    }
}

// ====== Coin Metadata Types ======

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinMetadata {
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[allow(dead_code)]
    id: Option<Value>,
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPage {
    pub data: Vec<SuiEvent>,
    pub next_cursor: Option<EventId>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuiEvent {
    pub id: EventId,
    pub package_id: Option<String>,
    pub transaction_module: Option<String>,
    pub sender: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub parsed_json: Option<Value>,
    pub bcs: Option<String>,
    pub timestamp_ms: Option<String>,
}

impl SuiEvent {
    #[allow(dead_code)]
    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp_ms
            .as_ref()
            .and_then(|ts| ts.parse::<u64>().ok())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventId {
    pub tx_digest: String,
    pub event_seq: String,
}

impl EventId {
    pub fn to_cursor(&self) -> String {
        format!("{}:{}", self.tx_digest, self.event_seq)
    }

    pub fn from_cursor_str(cursor: &str) -> Option<Self> {
        let (tx_digest, event_seq) = cursor.split_once(':')?;
        Some(Self {
            tx_digest: tx_digest.to_string(),
            event_seq: event_seq.to_string(),
        })
    }
}

// ====== Object Response Types ======

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectResponse {
    pub data: Option<ObjectData>,
    pub error: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectData {
    pub object_id: String,
    #[serde(rename = "type")]
    pub object_type: Option<String>,
    pub display: Option<DisplayData>,
    pub content: Option<ObjectContent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayData {
    pub data: Option<std::collections::HashMap<String, String>>,
    pub error: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectContent {
    pub data_type: Option<String>,
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub fields: Option<Value>,
}

impl ObjectData {
    /// Get display field value (e.g., "name", "image_url", "description")
    pub fn get_display_field(&self, field: &str) -> Option<String> {
        self.display
            .as_ref()
            .and_then(|d| d.data.as_ref())
            .and_then(|data| data.get(field).cloned())
    }

    /// Get NFT name from display or content fields
    pub fn get_name(&self) -> Option<String> {
        // Try display fields first
        if let Some(name) = self.get_display_field("name") {
            return Some(name);
        }
        // Fallback to content fields
        self.content
            .as_ref()
            .and_then(|c| c.fields.as_ref())
            .and_then(|f| f.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get NFT image_url from display or content fields
    pub fn get_image_url(&self) -> Option<String> {
        // Try display fields first
        if let Some(url) = self.get_display_field("image_url") {
            return Some(url);
        }
        // Fallback to content fields
        self.content
            .as_ref()
            .and_then(|c| c.fields.as_ref())
            .and_then(|f| f.get("image_url").or_else(|| f.get("url")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}
