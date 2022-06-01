#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Couldn't parse from json: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Hex decode error: {0}")]
    HexDecodeError(#[from] hex::FromHexError),
    #[error("Signature error: {0}")]
    EcdsaError(#[from] secp256k1::Error),
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Insufficient funds")]
    NoFunds,
    #[error("Another transaction is still pending: {0}")]
    PendingTx(web3::Error),
    #[error("Transaction failed: {0}")]
    Web3Error(#[from] web3::Error),
    #[error("Couldn't encode ABI: {0}")]
    AbiError(#[from] web3::ethabi::Error),
    #[error("Error: {0}")]
    GenericError(std::string::String),
}
