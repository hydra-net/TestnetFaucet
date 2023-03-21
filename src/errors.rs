use ethers::signers::LocalWallet;

use crate::json_rpc::EvmJsonRpcClient;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Couldn't parse from json: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Insufficient funds")]
    NoFunds,
    #[error("Invalid provider url: {0}")]
    InvalidProviderUrl(String),
    #[error("Http client error: {0}")]
    HttpClient(#[from] ethers::providers::HttpClientError),
    #[error("Ws client error: {0}")]
    WsClient(#[from] ethers::providers::WsClientError),
    #[error("Signer middleware error: {0}")]
    SignerMiddleware(
        #[from]
        ethers::prelude::signer::SignerMiddlewareError<
            ethers::providers::Provider<EvmJsonRpcClient>,
            LocalWallet,
        >,
    ),
    #[error("Contract error: {0}")]
    Contract(
        #[from]
        ethers::prelude::ContractError<
            ethers::prelude::signer::SignerMiddleware<
                ethers::providers::Provider<EvmJsonRpcClient>,
                LocalWallet,
            >,
        >,
    ),
    #[error("Lnd error: {0}")]
    Lnd(String),
}
