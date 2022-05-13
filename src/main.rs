mod erc20_abi;
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
                let coin = coin_address[0].replace(" ", "").to_uppercase(); // allow spaces and lowercase coin
                let address = coin_address[1].replace(" ", "");

                let config = CONFIG.lock().await;

                let response_msg: String;

                if let Some(coin_config) = config.coins.get(&coin) {
                    // TODO: validate address

                    let mut cache = CACHE.lock().await;

                    let mut user = match cache.get(&msg.author.id.0) {
                        Some(user) => user.to_owned(),
                        None => HashMap::new(),
                    };

                    let coin_timestamp = match user.get(&coin) {
                        Some(timestamp) => timestamp.to_owned(),
                        None => 0,
                    };

                    let current_timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs();

                    // check if enough hours had elapsed from last request
                    if current_timestamp > coin_timestamp + config.limit * 3600 {
                        // TODO: send coins

                        let tx_res = match coin.as_str() {
                            "BTC" => {
                                lnd_send(
                                    &config.btc_url,
                                    MACAROONS.lock().await.btc.to_owned(),
                                    address,
                                    coin_config.amount,
                                )
                                .await
                            }
                            "LTC" => {
                                lnd_send(
                                    &config.ltc_url,
                                    MACAROONS.lock().await.ltc.to_owned(),
                                    address,
                                    coin_config.amount,
                                )
                                .await
                            }
                            "ETH" => {
                                eth_send_transaction(
                                    config.eth_provider.to_owned(),
                                    Address::from_str(&config.eth_address).unwrap(),
                                    config.eth_privkey.to_owned(),
                                    address,
                                    coin_config.amount,
                                )
                                .await
                            }
                            _ => {
                                erc20_send_transaction(
                                    coin_config.to_owned(),
                                    config.eth_provider.to_owned(),
                                    Address::from_str(&config.eth_address).unwrap(),
                                    config.eth_privkey.to_owned(),
                                    address,
                                    coin_config.amount,
                                )
                                .await
                            }
                        };

                        match tx_res {
                            Ok(txid) => {
                                user.insert(coin.to_owned(), current_timestamp);
                                let mut new_cache = HashMap::new();
                                new_cache.insert(msg.author.id.0, user);
                                *cache = new_cache;

                                let explorer = match coin.as_str() {
                                    "BTC" => {
                                        format!(
                                            "https://www.blockchain.com/btc-testnet/tx/{}",
                                            txid
                                        )
                                    }
                                    "LTC" => {
                                        format!(
                                            "https://blockexplorer.one/litecoin/testnet/tx/{}",
                                            txid
                                        )
                                    }
                                    _ => format!("https://rinkeby.etherscan.io/tx/{}", txid),
                                };

                                response_msg =
                                    format!("Sent {} {}! {}", coin_config.amount, coin, explorer);
                            }
                            Err(error) => {
                                println!("{}", error);
                                response_msg = error;
                            }
                        }
                    } else {
                        let remaining = coin_timestamp + config.limit * 3600 - current_timestamp;
                        let hours = remaining / 3600;
                        let mins = (remaining - hours * 3600) / 60;
                        response_msg = format!(
                            "Please wait another {}h{}m before requesting new {}!",
                            hours, mins, coin
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
