#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Couldn't parse from json: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
    #[error("Signature error: {0}")]
    Ecdsa(#[from] secp256k1::Error),
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Insufficient funds")]
    NoFunds,
    #[error("Another transaction is still pending: {0}")]
    PendingTx(web3::Error),
    #[error("Transaction failed: {0}")]
    Web3(#[from] web3::Error),
    #[error("Couldn't encode ABI: {0}")]
    Abi(#[from] web3::ethabi::Error),
    #[error("Error: {0}")]
    Generic(std::string::String),
}
