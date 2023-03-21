use serde::Deserialize;
use serenity::model::prelude::UserId;
use std::{collections::HashMap, fmt::Display};

#[derive(Default, Clone, Deserialize)]
pub struct Config {
    pub token: String,
    pub eth_mnemonic: String,
    pub btc_url: String,
    pub ltc_url: String,
    pub btc_macaroon_dir: String,
    pub ltc_macaroon_dir: String,
    pub limit: u64,
    pub providers: HashMap<String, String>,
    pub coins: HashMap<String, ConfigCoin>,
}

#[derive(Clone, Deserialize)]
pub struct ConfigCoin {
    pub amount: f64,
    pub network: Network,
    #[serde(default)]
    pub contract: String,
    pub decimals: u32,
}

#[derive(Clone)]
pub struct Coin {
    pub name: String,
    pub amount: f64,
    pub network: Network,
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

impl Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Lightning => write!(f, "lightning"),
            Network::Ethereum => write!(f, "ethereum"),
            Network::Arbitrum => write!(f, "arbitrum"),
        }
    }
}

#[derive(Default, Clone, Deserialize)]
#[serde(default)]
pub struct Macaroon {
    pub btc: String,
    pub ltc: String,
}

pub type Cache = HashMap<UserId, HashMap<String, u64>>; // user_id -> coin -> timestamp
