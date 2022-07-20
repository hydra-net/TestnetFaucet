mod erc20_abi;
mod errors;
mod lightning_structs;
mod send;
mod structs;
mod utils;

#[macro_use]
extern crate lazy_static;

use config_file::FromConfigFile;
use send::erc20_send_transaction;
use send::eth_send_transaction;
use send::lnd_send;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use structs::Network;
use structs::{Cache, Config, Macaroon};
use web3::ethabi::Address;

struct Handler;

lazy_static! {
    static ref CACHE: Mutex<Cache> = Mutex::new(HashMap::new());
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::default());
    static ref MACAROONS: Mutex<Macaroon> = Mutex::new(Macaroon::default());
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if !msg.author.bot {
            let coin_address: Vec<&str> = msg.content.split("-").collect();

            if coin_address.len() == 2 {
                let coin_name = coin_address[0].replace(" ", "").to_uppercase(); // allow spaces and lowercase coin
                let address = coin_address[1].replace(" ", "");

                let config = CONFIG.lock().await;

                let response_msg: String;

                if let Some(coin) = config.coins.get(&coin_name) {
                    let mut cache = CACHE.lock().await;

                    let mut user = match cache.get(&msg.author.id.0) {
                        Some(user) => user.to_owned(),
                        None => HashMap::new(),
                    };

                    let coin_timestamp = match user.get(&coin_name) {
                        Some(timestamp) => timestamp.to_owned(),
                        None => 0,
                    };

                    let current_timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs();

                    // check if enough hours had elapsed from last request
                    if current_timestamp > coin_timestamp + config.limit * 3600 {
                        let tx_res = match coin.network {
                            Network::Lightning => {
                                let (url, macaroon) = match coin_name.as_str() {
                                    "BTC" => (
                                        config.btc_url.to_owned(),
                                        MACAROONS.lock().await.btc.to_owned(),
                                    ),
                                    "LTC" => (
                                        config.ltc_url.to_owned(),
                                        MACAROONS.lock().await.ltc.to_owned(),
                                    ),
                                    _ => {
                                        return;
                                    }
                                };

                                lnd_send(&coin, &url, &macaroon, &address, coin.amount).await
                            }
                            Network::Ethereum => match coin_name.as_str() {
                                "ETH" => {
                                    eth_send_transaction(
                                        &coin,
                                        config.providers.get(&coin.network.to_string()).unwrap(),
                                        Address::from_str(&config.eth_address).unwrap(),
                                        &config.eth_privkey,
                                        &address,
                                        coin.amount,
                                    )
                                    .await
                                }
                                _ => {
                                    erc20_send_transaction(
                                        &coin,
                                        config.providers.get(&coin.network.to_string()).unwrap(),
                                        Address::from_str(&config.eth_address).unwrap(),
                                        &config.eth_privkey,
                                        &address,
                                        coin.amount,
                                    )
                                    .await
                                }
                            },
                            Network::Arbitrum => match coin_name.as_str() {
                                "AETH" => {
                                    eth_send_transaction(
                                        &coin,
                                        config.providers.get(&coin.network.to_string()).unwrap(),
                                        Address::from_str(&config.eth_address).unwrap(),
                                        &config.eth_privkey,
                                        &address,
                                        coin.amount,
                                    )
                                    .await
                                }
                                _ => {
                                    erc20_send_transaction(
                                        &coin,
                                        config.providers.get(&coin.network.to_string()).unwrap(),
                                        Address::from_str(&config.eth_address).unwrap(),
                                        &config.eth_privkey,
                                        &address,
                                        coin.amount,
                                    )
                                    .await
                                }
                            },
                        };

                        match tx_res {
                            Ok(txid) => {
                                user.insert(coin_name.to_owned(), current_timestamp);
                                let mut new_cache = HashMap::new();
                                new_cache.insert(msg.author.id.0, user);
                                *cache = new_cache;

                                let explorer = match coin.network {
                                    Network::Lightning => match coin_name.as_str() {
                                        "BTC" => format!(
                                            "https://www.blockchain.com/btc-testnet/tx/{}",
                                            txid
                                        ),
                                        "LTC" => format!(
                                            "https://blockexplorer.one/litecoin/testnet/tx/{}",
                                            txid
                                        ),
                                        _ => {
                                            return;
                                        }
                                    },
                                    Network::Ethereum => {
                                        format!("https://rinkeby.etherscan.io/tx/{}", txid)
                                    }
                                    Network::Arbitrum => {
                                        format!("https://testnet.arbiscan.io/tx/{}", txid)
                                    }
                                };

                                response_msg =
                                    format!("Sent {} {}! {}", coin.amount, coin_name, explorer);
                            }
                            Err(error) => {
                                println!("{}", error);
                                response_msg = match error {
                                    errors::Error::InvalidAddress => String::from("Invalid address!"),
                                    errors::Error::NoFunds => String::from("Faucet out of funds!"),
                                    errors::Error::PendingTx(_) => String::from("Another transaction is still pending, retry in some minutes!"),
                                    _ => String::from("Transaction failed, retry later!"),
                                };
                            }
                        }
                    } else {
                        let remaining = coin_timestamp + config.limit * 3600 - current_timestamp;
                        let hours = remaining / 3600;
                        let mins = (remaining - hours * 3600) / 60;
                        response_msg = format!(
                            "Please wait another {}h{}m before requesting new {}!",
                            hours, mins, coin_name
                        );
                    }
                } else {
                    response_msg = "Coin not supported!".to_string();
                }

                let response = MessageBuilder::new()
                    .mention(&msg.author)
                    .push(response_msg)
                    .build();

                if let Err(why) = msg.channel_id.say(&context.http, &response).await {
                    println!("Error sending message: {:?}", why);
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let config =
        Config::from_config_file("config.toml").expect("config.toml not properly formatted");

    *CONFIG.lock().await = config.to_owned();

    let btc_macaroon = match std::fs::read(PathBuf::from(&config.btc_macaroon_dir)) {
        Ok(macaroon) => hex::encode(macaroon),
        Err(error) => {
            panic!("BTC admin.macaroon file not found!\nERROR: {:#?}", error);
        }
    };

    let ltc_macaroon = match std::fs::read(PathBuf::from(&config.ltc_macaroon_dir)) {
        Ok(macaroon) => hex::encode(macaroon),
        Err(error) => {
            panic!("LTC admin.macaroon file not found!\nERROR: {:#?}", error);
        }
    };

    let macaroons = Macaroon {
        btc: btc_macaroon,
        ltc: ltc_macaroon,
    };

    *MACAROONS.lock().await = macaroons;

    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&config.token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
