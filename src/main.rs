mod errors;
mod ierc20;
mod json_rpc;
mod lightning_structs;
mod send;
mod structs;
mod utils;

use config_file::FromConfigFile;
use ethers::signers::coins_bip39::English;
use ethers::signers::LocalWallet;
use ethers::signers::MnemonicBuilder;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use structs::Coin;
use structs::Network;
use structs::{Cache, Config, Macaroon};

struct Faucet {
    config: Config,
    cache: Mutex<Cache>,
    macaroon: Macaroon,
    eth_wallet: LocalWallet,
}

#[async_trait]
impl EventHandler for Faucet {
    async fn message(&self, context: Context, msg: Message) {
        if !msg.author.bot {
            let coin_address: Vec<&str> = msg.content.split('-').collect();

            if coin_address.len() == 2 {
                let coin_name = coin_address[0].replace(' ', "").to_uppercase(); // allow spaces and lowercase coin
                let address = coin_address[1].replace(' ', "");

                let response_msg: String;

                if let Some(config_coin) = self.config.coins.get(&coin_name) {
                    let coin = Coin {
                        name: coin_name.to_owned(),
                        amount: config_coin.amount,
                        decimals: config_coin.decimals,
                        contract: config_coin.contract.to_owned(),
                        network: config_coin.network.to_owned(),
                    };

                    let mut cache = self.cache.lock().await;

                    let coin_timestamp = match cache.get(&msg.author.id) {
                        Some(user) => match user.get(&coin_name) {
                            Some(timestamp) => timestamp.to_owned(),
                            None => 0,
                        },
                        None => 0,
                    };

                    let current_timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs();

                    // check if enough hours had elapsed from last request
                    if current_timestamp > coin_timestamp + self.config.limit * 3600 {
                        let tx_res = match coin.network {
                            Network::Lightning => self.lnd_send(&coin, &address, coin.amount).await,
                            Network::Ethereum => match coin_name.as_str() {
                                "ETH" => {
                                    self.eth_send_transaction(&coin, &address, coin.amount)
                                        .await
                                }
                                _ => {
                                    self.erc20_send_transaction(&coin, &address, coin.amount)
                                        .await
                                }
                            },
                            Network::Arbitrum => match coin_name.as_str() {
                                "AETH" => {
                                    self.eth_send_transaction(&coin, &address, coin.amount)
                                        .await
                                }
                                _ => {
                                    self.erc20_send_transaction(&coin, &address, coin.amount)
                                        .await
                                }
                            },
                        };

                        match tx_res {
                            Ok(txid) => {
                                match cache.get_mut(&msg.author.id) {
                                    Some(user) => {
                                        user.insert(coin_name.to_owned(), current_timestamp);
                                    }
                                    None => {
                                        let mut user = HashMap::new();
                                        user.insert(coin_name.to_owned(), current_timestamp);
                                        cache.insert(msg.author.id, user);
                                    }
                                }

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
                                        format!("https://goerli.etherscan.io/tx/{}", txid)
                                    }
                                    Network::Arbitrum => {
                                        format!(
                                            "https://goerli-rollup-explorer.arbitrum.io/tx/{}",
                                            txid
                                        )
                                    }
                                };

                                response_msg =
                                    format!("Sent {} {}! {}", coin.amount, coin_name, explorer);
                            }
                            Err(error) => {
                                println!("{}", error);
                                response_msg = match error {
                                    errors::Error::InvalidAddress => {
                                        String::from("Invalid address!")
                                    }
                                    errors::Error::NoFunds => String::from("Faucet out of funds!"),
                                    _ => {
                                        if error
                                            .to_string()
                                            .contains("replacement transaction underpriced")
                                        {
                                            String::from("Please wait for the previous transaction to be confirmed!")
                                        } else {
                                            String::from("Transaction failed, retry later!")
                                        }
                                    }
                                };
                            }
                        }
                    } else {
                        let remaining =
                            coin_timestamp + self.config.limit * 3600 - current_timestamp;
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

    let macaroon = Macaroon {
        btc: btc_macaroon,
        ltc: ltc_macaroon,
    };

    let eth_wallet = MnemonicBuilder::<English>::default()
        .phrase(&*config.eth_mnemonic)
        .build()
        .unwrap();

    let faucet = Faucet {
        config: config.clone(),
        cache: Mutex::new(HashMap::new()),
        macaroon,
        eth_wallet,
    };

    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&config.token, intents)
        .event_handler(faucet)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
