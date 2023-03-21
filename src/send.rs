use std::str::FromStr;
use std::sync::Arc;

use crate::errors::Error;
use crate::ierc20::IERC20;
use crate::json_rpc::EvmJsonRpcClient;
use crate::Faucet;

use super::lightning_structs;
use super::structs::Coin;
use super::utils::btc_to_sat;
use super::utils::eth_to_wei;
use ethers::prelude::SignerMiddleware;
use ethers::providers::Http;
use ethers::providers::Middleware;
use ethers::providers::Provider;
use ethers::providers::Ws;
use ethers::types::Address;
use ethers::types::Eip1559TransactionRequest;
use reqwest::header::HeaderMap;

impl Faucet {
    pub async fn lnd_send(&self, coin: &Coin, address: &str, amount: f64) -> Result<String, Error> {
        let (url, macaroon) = match coin.name.as_str() {
            "BTC" => (self.config.btc_url.to_owned(), self.macaroon.btc.to_owned()),
            "LTC" => (self.config.ltc_url.to_owned(), self.macaroon.ltc.to_owned()),
            _ => {
                return Err(Error::Lnd(format!(
                    "Coin {} is not supported by LND",
                    coin.name
                )));
            }
        };

        let request = lightning_structs::SendCoinsRequest {
            addr: address.to_string(),
            amount: btc_to_sat(amount, coin.decimals) as i64,
            ..Default::default()
        };

        let endpoint = "/v1/transactions";
        let url = url.to_owned() + endpoint;

        let mut headers = HeaderMap::new();
        headers.insert("Grpc-Metadata-macaroon", macaroon.parse().unwrap());

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()?;

        let body = serde_json::to_string(&request)?;

        let response = client.post(&url).body(body).send().await?;

        let response_message = response.text().await?;

        let message =
            match serde_json::from_str::<lightning_structs::SendCoinsResponse>(&response_message) {
                Ok(message) => message,
                Err(_) => {
                    println!("{}", response_message);
                    if response_message.contains("not valid for this network")
                        || response_message.contains("address")
                    {
                        return Err(Error::InvalidAddress);
                    } else if response_message.contains("insufficient") {
                        return Err(Error::NoFunds);
                    } else {
                        return Err(Error::Lnd(response_message));
                    }
                }
            };

        Ok(message.txid)
    }

    pub async fn eth_send_transaction(
        &self,
        coin: &Coin,
        to_address: &str,
        amount: f64,
    ) -> Result<String, Error> {
        let to_address = match Address::from_str(to_address) {
            Ok(addr) => addr,
            Err(_) => return Err(Error::InvalidAddress),
        };

        let provider_url = self
            .config
            .providers
            .get(&coin.network.to_string())
            .unwrap_or_else(|| panic!("No provider for network {}", coin.network));

        let provider = if provider_url.starts_with("http") {
            let http_client = Http::from_str(provider_url)
                .map_err(|_| Error::InvalidProviderUrl(provider_url.to_string()))?;
            Provider::new(EvmJsonRpcClient::Http(http_client))
        } else if provider_url.starts_with("ws") {
            let ws = Ws::connect(provider_url).await?;
            Provider::new(EvmJsonRpcClient::Ws(ws))
        } else {
            return Err(Error::InvalidProviderUrl(provider_url.to_string()));
        };

        let client =
            SignerMiddleware::new_with_provider_chain(provider, self.eth_wallet.clone()).await?;

        let tx = Eip1559TransactionRequest::new()
            .to(to_address)
            .value(eth_to_wei(amount, coin.decimals));

        let pending_tx = client.send_transaction(tx, None).await?;
        let tx_hash = pending_tx.tx_hash();

        Ok(format!("0x{}", hex::encode(tx_hash.as_bytes())))
    }

    pub async fn erc20_send_transaction(
        &self,
        coin: &Coin,
        to_address: &str,
        amount: f64,
    ) -> Result<String, Error> {
        let contract_addr = match Address::from_str(&coin.contract) {
            Ok(address) => address,
            Err(_) => {
                return Err(Error::InvalidAddress);
            }
        };

        let to_address = match Address::from_str(to_address) {
            Ok(address) => address,
            Err(_) => {
                return Err(Error::InvalidAddress);
            }
        };

        let provider_url = self
            .config
            .providers
            .get(&coin.network.to_string())
            .unwrap_or_else(|| panic!("No provider for network {}", coin.network));

        let provider = if provider_url.starts_with("http") {
            let http_client = Http::from_str(provider_url)
                .map_err(|_| Error::InvalidProviderUrl(provider_url.to_string()))?;
            Provider::new(EvmJsonRpcClient::Http(http_client))
        } else if provider_url.starts_with("ws") {
            let ws = Ws::connect(provider_url).await?;
            Provider::new(EvmJsonRpcClient::Ws(ws))
        } else {
            return Err(Error::InvalidProviderUrl(provider_url.to_string()));
        };

        let client =
            SignerMiddleware::new_with_provider_chain(provider, self.eth_wallet.clone()).await?;

        let erc20_contract = IERC20::new(contract_addr, Arc::new(client));
        let contract_tx = erc20_contract.transfer(to_address, eth_to_wei(amount, coin.decimals));
        let pending_tx = contract_tx.send().await?;

        let tx_hash = pending_tx.tx_hash();

        Ok(format!("0x{}", hex::encode(tx_hash.as_bytes())))
    }
}
