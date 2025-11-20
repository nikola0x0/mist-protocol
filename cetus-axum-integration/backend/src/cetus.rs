use anyhow::Result;
use serde::{Deserialize, Serialize};

const CETUS_API_BASE: &str = "https://api-sui.cetus.zone/v2/sui";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CetusPool {
    #[serde(rename = "swap_account")]
    pub swap_account: String,

    #[serde(rename = "symbol")]
    pub symbol: String,

    #[serde(rename = "token_a_address")]
    pub coin_a_address: String,

    #[serde(rename = "token_b_address")]
    pub coin_b_address: String,

    #[serde(rename = "fee", alias = "fee_rate")]
    pub fee_rate: String,

    #[serde(rename = "current_sqrt_price", default)]
    pub current_sqrt_price: String,

    #[serde(default)]
    pub tvl_in_usd: String,

    #[serde(default)]
    pub vol_in_usd_24h: String,
}

#[derive(Debug, Deserialize)]
struct CetusApiResponse {
    code: i32,
    msg: String,
    data: CetusPoolsData,
}

#[derive(Debug, Deserialize)]
struct CetusPoolsData {
    pools: Vec<CetusPool>,
}

pub struct CetusService;

impl CetusService {
    /// Fetch all available pools from Cetus API
    pub async fn fetch_pools(client: &reqwest::Client) -> Result<Vec<CetusPool>> {
        let url = format!("{}/pool_list", CETUS_API_BASE);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Cetus API returned error: {}", response.status());
        }

        let api_response: CetusApiResponse = response.json().await?;

        if api_response.code != 200 {
            anyhow::bail!("Cetus API error: {}", api_response.msg);
        }

        Ok(api_response.data.pools)
    }

    /// Find a specific pool by token pair
    pub async fn find_pool(
        client: &reqwest::Client,
        token_a: &str,
        token_b: &str,
    ) -> Result<Option<CetusPool>> {
        let pools = Self::fetch_pools(client).await?;

        // Normalize token addresses for comparison
        let token_a_lower = token_a.to_lowercase();
        let token_b_lower = token_b.to_lowercase();

        // Find pool matching the token pair (in either direction)
        let pool = pools.into_iter().find(|p| {
            let coin_a_lower = p.coin_a_address.to_lowercase();
            let coin_b_lower = p.coin_b_address.to_lowercase();

            (coin_a_lower == token_a_lower && coin_b_lower == token_b_lower) ||
            (coin_a_lower == token_b_lower && coin_b_lower == token_a_lower)
        });

        Ok(pool)
    }

    /// Find pools containing a specific token
    pub async fn find_pools_by_token(
        client: &reqwest::Client,
        token: &str,
    ) -> Result<Vec<CetusPool>> {
        let pools = Self::fetch_pools(client).await?;
        let token_lower = token.to_lowercase();

        let matching_pools: Vec<CetusPool> = pools
            .into_iter()
            .filter(|p| {
                p.coin_a_address.to_lowercase() == token_lower ||
                p.coin_b_address.to_lowercase() == token_lower
            })
            .collect();

        Ok(matching_pools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_pools() {
        let client = reqwest::Client::new();
        let pools = CetusService::fetch_pools(&client).await;

        assert!(pools.is_ok());
        let pools = pools.unwrap();
        assert!(!pools.is_empty());
        println!("Fetched {} pools", pools.len());

        // Print first few pools for verification
        for pool in pools.iter().take(3) {
            println!("Pool: {} - {}", pool.symbol, pool.swap_account);
        }
    }
}
