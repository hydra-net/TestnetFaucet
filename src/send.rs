use crate::errors::Error;

use super::lightning_structs;
use super::structs::Coin;
use super::utils::btc_to_sat;
use super::utils::eth_to_wei;
use reqwest::header::HeaderMap;
use secp256k1::SecretKey;
use std::str::FromStr;
use web3::ethabi::Token;
use web3::types::Address;
use web3::types::Bytes;
use web3::types::CallRequest;
use web3::types::TransactionParameters;

pub async fn lnd_send(
    url: &String,
    macaroon: String,
    address: String,
    amount: f64,
) -> Result<String, Error> {
    let request = lightning_structs::SendCoinsRequest {
        addr: address,
        amount: btc_to_sat(amount, 8) as i64,
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
                if response_message.contains("not valid for this network") {
                    return Err(Error::InvalidAddress);
                } else if response_message.contains("address") {
                    return Err(Error::InvalidAddress);
                } else if response_message.contains("insufficient") {
                    return Err(Error::NoFunds);
                } else {
                    return Err(Error::GenericError(response_message));
                }
            }
        };

    return Ok(message.txid);
}

pub async fn eth_send_transaction(
    provider: String,
    evm_address: Address,
    evm_privkey: String,
    address: String,
    amount: f64,
) -> Result<String, Error> {
    let to_address: Address;

    match Address::from_str(&address) {
        Ok(addr) => to_address = addr,
        Err(_) => return Err(Error::InvalidAddress),
    }

    let websocket = match web3::transports::WebSocket::new(&provider).await {
        Ok(websocket) => websocket,
        Err(error) => {
            return Err(Error::GenericError(format!(
                "Couldn't connect to the provider!\nERROR: {:?}",
                error
            )));
        }
    };

    let web3s = web3::Web3::new(websocket);

    let gas_tx = CallRequest {
        from: Some(evm_address),
        to: Some(to_address),
        gas: None,
        gas_price: None,
        value: Some(eth_to_wei(amount, 18)),
        data: None,
        transaction_type: None,
        access_list: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let gas = match web3s.eth().estimate_gas(gas_tx, None).await {
        Ok(gas) => gas,
        Err(error) => {
            if error.to_string().contains("insufficient") {
                return Err(Error::NoFunds);
            } else {
                return Err(Error::GenericError(format!(
                    "Couldn't estimate gas!\nERROR: {:#}",
                    error
                )));
            }
        }
    };

    let tx = TransactionParameters {
        to: Some(to_address),
        gas: gas,
        value: eth_to_wei(amount, 18),
        ..Default::default()
    };

    let prvk = SecretKey::from_str(&evm_privkey.replace("0x", ""))?;

    let signed_tx = web3s.accounts().sign_transaction(tx, &prvk).await?;

    let txid = match web3s
        .eth()
        .send_raw_transaction(signed_tx.raw_transaction)
        .await
    {
        Ok(txid) => "0x".to_string() + &hex::encode(txid.as_bytes().to_vec()),
        Err(error) => {
            if error.to_string().contains("replacement") {
                return Err(Error::PendingTx(error));
            } else {
                return Err(Error::Web3Error(error));
            }
        }
    };

    return Ok(txid);
}

pub async fn erc20_send_transaction(
    coin: Coin,
    provider: String,
    evm_address: Address,
    evm_privkey: String,
    address: String,
    amount: f64,
) -> Result<String, Error> {
    let websocket = match web3::transports::WebSocket::new(&provider).await {
        Ok(websocket) => websocket,
        Err(error) => {
            return Err(Error::GenericError(format!(
                "Couldn't connect to the provider!\nERROR: {:?}",
                error
            )));
        }
    };

    let web3s = web3::Web3::new(websocket);

    let contract_addr = match Address::from_str(&coin.contract) {
        Ok(address) => address,
        Err(_) => {
            return Err(Error::InvalidAddress);
        }
    };

    let to_address = match Address::from_str(&address) {
        Ok(address) => address,
        Err(_) => {
            return Err(Error::InvalidAddress);
        }
    };

    let transfer_function = super::erc20_abi::transfer();

    let data = match transfer_function.encode_input(&[
        Token::Address(to_address),
        Token::Uint(eth_to_wei(amount, coin.decimals)),
    ]) {
        Ok(data) => Bytes::from(data),
        Err(error) => {
            return Err(Error::AbiError(error));
        }
    };

    let gas_tx = CallRequest {
        from: Some(evm_address),
        to: Some(contract_addr),
        gas: None,
        gas_price: None,
        value: None,
        data: Some(data.to_owned()),
        transaction_type: None,
        access_list: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    let gas = match web3s.eth().estimate_gas(gas_tx, None).await {
        Ok(gas) => gas,
        Err(error) => {
            if error.to_string().contains("insufficient") {
                return Err(Error::NoFunds);
            } else {
                return Err(Error::GenericError(format!(
                    "Couldn't estimate gas!\nERROR: {:#}",
                    error
                )));
            }
        }
    };

    let tx = TransactionParameters {
        to: Some(contract_addr),
        data: data,
        gas: gas,
        ..Default::default()
    };

    let prvk = SecretKey::from_str(&evm_privkey.replace("0x", ""))?;

    let signed_tx = web3s.accounts().sign_transaction(tx, &prvk).await?;

    let txid = match web3s
        .eth()
        .send_raw_transaction(signed_tx.raw_transaction)
        .await
    {
        Ok(txid) => "0x".to_string() + &hex::encode(txid.as_bytes().to_vec()),
        Err(error) => {
            if error.to_string().contains("replacement") {
                return Err(Error::PendingTx(error));
            } else {
                return Err(Error::Web3Error(error));
            }
        }
    };

    Ok(txid)
}
