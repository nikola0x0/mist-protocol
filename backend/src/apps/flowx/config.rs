use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    // Network
    pub sui_network: String,
    
    // Wallet
    pub private_key: String,
    
    // FlowX Contract
    pub flowx_package_id: String,
    pub pool_registry_id: String,
    pub position_registry_id: String,
    pub versioned_id: String,
    
    // Your Token
    pub your_token_type: String,
    pub your_token_metadata_id: String,
    pub your_token_decimals: u8,
    
    // SUI
    pub sui_metadata_id: String,
    
    // Server
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        
        let your_token_package = env::var("YOUR_TOKEN_PACKAGE_ID")
            .context("YOUR_TOKEN_PACKAGE_ID not set")?;
        let your_token_module = env::var("YOUR_TOKEN_MODULE")
            .context("YOUR_TOKEN_MODULE not set")?;
        let your_token_name = env::var("YOUR_TOKEN_NAME")
            .context("YOUR_TOKEN_NAME not set")?;
        
        // Construct full token type: 0xPACKAGE::module::NAME
        let your_token_type = format!("{}::{}::{}", your_token_package, your_token_module, your_token_name);
        
        Ok(Config {
            sui_network: env::var("SUI_NETWORK").unwrap_or_else(|_| "testnet".to_string()),
            private_key: env::var("FLOWX_PRIVATE_KEY")
                .or_else(|_| env::var("PRIVATE_KEY"))
                .context("FLOWX_PRIVATE_KEY or PRIVATE_KEY not set")?,
            flowx_package_id: env::var("FLOWX_PACKAGE_ID").context("FLOWX_PACKAGE_ID not set")?,
            pool_registry_id: env::var("POOL_REGISTRY_ID").context("POOL_REGISTRY_ID not set")?,
            position_registry_id: env::var("POSITION_REGISTRY_ID").context("POSITION_REGISTRY_ID not set")?,
            versioned_id: env::var("VERSIONED_ID").context("VERSIONED_ID not set")?,
            your_token_type,
            your_token_metadata_id: env::var("YOUR_TOKEN_METADATA_ID").context("YOUR_TOKEN_METADATA_ID not set")?,
            your_token_decimals: env::var("YOUR_TOKEN_DECIMALS")
                .unwrap_or_else(|_| "9".to_string())
                .parse()
                .context("Invalid YOUR_TOKEN_DECIMALS")?,
            sui_metadata_id: env::var("SUI_METADATA_ID")
                .unwrap_or_else(|_| "0x9258181f5ceac8dbffb7030890243caed69a9599d2886d957a9cb7656af3bdb3".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Invalid PORT")?,
        })
    }
    
    pub fn sui_rpc_url(&self) -> String {
        match self.sui_network.as_str() {
            "mainnet" => "https://fullnode.mainnet.sui.io:443".to_string(),
            "testnet" => "https://fullnode.testnet.sui.io:443".to_string(),
            "devnet" => "https://fullnode.devnet.sui.io:443".to_string(),
            "localnet" => "http://127.0.0.1:9000".to_string(),
            url => url.to_string(), // Custom URL
        }
    }
    
    pub fn sui_type(&self) -> String {
        "0x2::sui::SUI".to_string()
    }
}
