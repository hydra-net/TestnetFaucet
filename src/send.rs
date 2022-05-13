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
) -> Result<String, String> {
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
        .build()
        .unwrap();

    let body = serde_json::to_string(&request).unwrap();

    let response = match client.post(&url).body(body).send().await {
        Ok(response) => response,
        Err(error) => {
            return Err(error.to_string());
        }
    };

    let response_message = response.text().await.unwrap();

    let message =
        match serde_json::from_str::<lightning_structs::SendCoinsResponse>(&response_message) {
            Ok(message) => message,
            Err(_) => {
                println!("{}", response_message);
                let err_message: String;
                if response_message.contains("not valid for this network") {
                    err_message = "Address not valid for this network!".to_string();
                } else if response_message.contains("address") {
                    err_message = "Address not valid!".to_string();
                } else if response_message.contains("insufficient") {
                    err_message = "Insufficient funds!".to_string();
                } else {
                    err_message = "Couldn't send onchain coins!".to_string();
                }
                return Err(err_message);
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
) -> Result<String, String> {
    let to_address: Address;

    match Address::from_str(&address) {
        Ok(addr) => to_address = addr,
        Err(_) => {
            return Err("Invalid address!".to_string());
        }
    }

    let websocket = match web3::transports::WebSocket::new(&provider).await {
        Ok(websocket) => websocket,
        Err(error) => {
            return Err(format!(
                "Couldn't connect to the provider!\nERROR: {:?}",
                error
            ));
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
                return Err("Insufficient funds!".to_string());
            } else {
                return Err(format!("Couldn't estimate gas!\nERROR: {:#}", error));
            }
        }
    };

    let tx = TransactionParameters {
        to: Some(to_address),
        gas: gas,
        value: eth_to_wei(amount, 18),
        ..Default::default()
    };

    let prvk = SecretKey::from_str(&evm_privkey.replace("0x", "")).unwrap();

    let signed_tx = match web3s.accounts().sign_transaction(tx, &prvk).await {
        Ok(signed_tx) => signed_tx,
        Err(error) => {
            return Err(format!("Couldn't sign transaction!\nERROR: {:?}", &error));
        }
    };

    let txid = match web3s
        .eth()
        .send_raw_transaction(signed_tx.raw_transaction)
        .await
    {
        Ok(txid) => "0x".to_string() + &hex::encode(txid.as_bytes().to_vec()),
        Err(error) => {
            if error.to_string().contains("replacement") {
                return Err(format!(
                    "Please wait for the previous transaction to be accepted by the blockchain!"
                ));
            } else {
                return Err(format!("Couldn't send transaction!\nERROR: {:#}", &error));
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
) -> Result<String, String> {
    let websocket = match web3::transports::WebSocket::new(&provider).await {
        Ok(websocket) => websocket,
        Err(error) => {
            return Err(format!(
                "Couldn't connect to the provider!\nERROR: {:?}",
                error
            ));
        }
    };

    let web3s = web3::Web3::new(websocket);

    let contract_addr = Address::from_str(&coin.contract).unwrap();

    let to_address = match Address::from_str(&address) {
        Ok(address) => address,
        Err(_) => {
            return Err("Invalid address!".to_string());
        }
    };

    let transfer_function = super::erc20_abi::transfer();

    let data = match transfer_function.encode_input(&[
        Token::Address(to_address),
        Token::Uint(eth_to_wei(amount, coin.decimals)),
    ]) {
        Ok(data) => Bytes::from(data),
        Err(error) => {
            return Err(format!("Couldn't encode ABI\nERROR: {:?}", error));
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
                return Err("Insufficient funds!".to_string());
            } else {
                return Err(format!("Couldn't estimate gas!\nERROR: {:?}", error));
            }
        }
    };

    let tx = TransactionParameters {
        to: Some(contract_addr),
        data: data,
        gas: gas,
        ..Default::default()
    };

    let prvk = SecretKey::from_str(&evm_privkey.replace("0x", "")).unwrap();

    let signed_tx = match web3s.accounts().sign_transaction(tx, &prvk).await {
        Ok(signed_tx) => signed_tx,
        Err(error) => {
            return Err(format!("Couldn't sign transaction!\nERROR: {:?}", error));
        }
    };

    let txid = match web3s
        .eth()
        .send_raw_transaction(signed_tx.raw_transaction)
        .await
    {
        Ok(txid) => "0x".to_string() + &hex::encode(txid.as_bytes().to_vec()),
        Err(error) => {
            if error.to_string().contains("replacement") {
                return Err(format!(
                    "Please wait for the previous transaction to be accepted by the blockchain!"
                ));
            } else {
                return Err(format!("Couldn't send transaction!\nERROR: {:#}", &error));
            }
        }
    };

    Ok(txid)
}
