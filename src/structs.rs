use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Clone, Deserialize)]
pub struct Config {
    pub token: String,
    pub eth_privkey: String,
    pub eth_address: String,
    pub btc_url: String,
    pub ltc_url: String,
    pub btc_macaroon_dir: String,
    pub ltc_macaroon_dir: String,
    pub limit: u64,
    pub providers: HashMap<String, String>,
    pub coins: HashMap<String, Coin>,
}

#[derive(Clone, Deserialize)]
pub struct Coin {
    pub amount: f64,
    pub network: Network,
    #[serde(default)]
    pub contract: String,
    pub decimals: u32,
}

#[derive(Clone, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Network {
    Lightning,
    Ethereum,
    Arbitrum,
}

impl ToString for Network {
    fn to_string(&self) -> String {
        match self {
            Network::Lightning => "lightning".to_string(),
            Network::Ethereum => "ethereum".to_string(),
            Network::Arbitrum => "arbitrum".to_string(),
        }
    }
}

#[derive(Default, Clone, Deserialize)]
#[serde(default)]
pub struct Macaroon {
    pub btc: String,
    pub ltc: String,
}

pub type Cache = HashMap<u64, HashMap<String, u64>>; // user_id -> coin -> timestamp
