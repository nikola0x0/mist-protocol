use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            _ => Err(format!("Invalid network: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub network: Network,
    pub rpc_url: String,
    pub api_base: String,
    pub clmm_package: String,
    pub integrate_package: String,
    pub global_config: String,
    pub clock_address: String,
}

impl AppConfig {
    pub fn new(network: Network) -> Self {
        match network {
            Network::Mainnet => Self {
                network,
                rpc_url: "https://fullnode.mainnet.sui.io:443".to_string(),
                api_base: "https://api-sui.cetus.zone/v2/sui".to_string(),
                clmm_package: "0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb"
                    .to_string(),
                integrate_package: "0xb2db7142fa83210a7d78d9c12ac49c043b3cbbd482224fea6e3da00aa5a5ae2d"
                    .to_string(),
                global_config: "0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f"
                    .to_string(),
                clock_address: "0x6".to_string(),
            },
            Network::Testnet => Self {
                network,
                rpc_url: "https://fullnode.testnet.sui.io:443".to_string(),
                api_base: "https://api-sui.cetus.zone/v2/sui".to_string(),
                clmm_package: "0x5372d555ac734e272659136c2a0cd3227f9b92de67c80dc11250307268af2db8"
                    .to_string(),
                integrate_package: "0x19dd42e05fa6c9988a60d30686ee3feb776672b5547e328d6dab16563da65293"
                    .to_string(),
                global_config: "0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca"
                    .to_string(),
                clock_address: "0x6".to_string(),
            },
        }
    }
}

// Contract addresses reference:
//
// MAINNET:
// - CLMM Package: 0x1eabed72c53feb3805120a081dc15963c204dc8d091542592abaf7a35689b2fb
// - Integrate: 0x996c4d9480708fb8b92aa7acf819fb0497b5ec8e65ba06601cae2fb6db3312c3
// - Config: 0x95b8d278b876cae22206131fb9724f701c9444515813042f54f0a426c9a3bc2f
// - Global Config: 0xdaa46292632c3c4d8f31f23ea0f9b36a28ff3677e9684980e4438403a67a3d8f
//
// TESTNET:
// - CLMM Package: 0x5372d555ac734e272659136c2a0cd3227f9b92de67c80dc11250307268af2db8
// - Integrate: 0x19dd42e05fa6c9988a60d30686ee3feb776672b5547e328d6dab16563da65293
// - Config: 0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca
// - Global Config: 0xf5ff7d5ba73b581bca6b4b9fa0049cd320360abd154b809f8700a8fd3cfaf7ca
//
// Token addresses:
// - CETUS: 0x6864a6f921804860930db6ddbe2e16acdf8504495ea7481637a1c8b9a8fe54b::cetus::CETUS
// - xCETUS: 0x9e69acc50ca03bc943c4f7c5304c2a6002d507b51c11913b247159c60422c606::xcetus::XCETUS
//
// Always check official docs for latest addresses:
// https://cetus-1.gitbook.io/cetus-developer-docs
