use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Clone, Deserialize)]
pub struct Config {
    pub token: String,
    pub eth_provider: String,
    pub eth_privkey: String,
    pub eth_address: String,
    pub btc_url: String,
    pub ltc_url: String,
    pub btc_macaroon_dir: String,
    pub ltc_macaroon_dir: String,
    pub limit: u64,
    pub coins: HashMap<String, Coin>,
}

#[derive(Default, Clone, Deserialize)]
#[serde(default)]
pub struct Coin {
    pub amount: f64,
    pub contract: String,
    pub decimals: u32,
}

#[derive(Default, Clone, Deserialize)]
#[serde(default)]
pub struct Macaroon {
    pub btc: String,
    pub ltc: String,
}

pub type Cache = HashMap<u64, HashMap<String, u64>>; // user_id -> coin -> timestamp
