//! FlowX DEX configuration

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    // Network
    pub sui_network: String,

    // FlowX Contract IDs (Testnet)
    pub flowx_package_id: String,
    pub pool_registry_id: String,
    pub position_registry_id: String,
    pub versioned_id: String,

    // SUI metadata
    pub sui_metadata_id: String,
}

impl Config {
    /// Create config for testnet
    pub fn testnet() -> Self {
        Self {
            sui_network: "testnet".to_string(),
            // FlowX Testnet addresses (from feat/unified-BE-cetus-flowx branch)
            flowx_package_id: "0x6cc1ce379acd35203f856f1dd0e063023caf091c47ce19b4695299de8b5fcb17".to_string(),
            pool_registry_id: "0xe59d16a0427a1ad98302eda383025d342d555ff9a98113f421c2184bdee1963e".to_string(),
            position_registry_id: "0x59ddf1ebeab7c1d48c6ad87b69fecb38bfc0524e5cf32cdbf51c13e15f2c37c3".to_string(),
            versioned_id: "0xf7eacab72d4a09da34ceb38922c21d7c48cb6bbedb5f1c57899f5c782abe1b5c".to_string(),
            sui_metadata_id: "0x9258181f5ceac8dbffb7030890243caed69a9599d2886d957a9cb7656af3bdb3".to_string(),
        }
    }

    /// Create config for mainnet
    pub fn mainnet() -> Self {
        Self {
            sui_network: "mainnet".to_string(),
            // FlowX Mainnet addresses - update these with actual mainnet addresses
            flowx_package_id: "0x25929e75f39b0ecacfc97b6fcaeef853cf9c5391c63e9107ceefd69e89e8f92d".to_string(),
            pool_registry_id: "0xb9628e6ea6feeba7e313e87a5665edfd55b14a69c93d2a1f26b8a4e64c5c06c8".to_string(),
            position_registry_id: "0x59ddf1ebeab7c1d48c6ad87b69fecb38bfc0524e5cf32cdbf51c13e15f2c37c3".to_string(),
            versioned_id: "0x67624a1533b5aff5d9a6c6c8eaf43b4cd5569f7e4b9850b4a6472d9a76f33983".to_string(),
            sui_metadata_id: "0x9258181f5ceac8dbffb7030890243caed69a9599d2886d957a9cb7656af3bdb3".to_string(),
        }
    }

    /// Create from network string
    pub fn from_network(network: &str) -> Result<Self> {
        match network.to_lowercase().as_str() {
            "testnet" => Ok(Self::testnet()),
            "mainnet" => Ok(Self::mainnet()),
            _ => Err(anyhow::anyhow!("Unknown network: {}", network)),
        }
    }

    pub fn sui_rpc_url(&self) -> String {
        match self.sui_network.as_str() {
            "mainnet" => "https://fullnode.mainnet.sui.io:443".to_string(),
            "testnet" => "https://fullnode.testnet.sui.io:443".to_string(),
            "devnet" => "https://fullnode.devnet.sui.io:443".to_string(),
            "localnet" => "http://127.0.0.1:9000".to_string(),
            url => url.to_string(),
        }
    }

    pub fn sui_type(&self) -> String {
        "0x2::sui::SUI".to_string()
    }
}
