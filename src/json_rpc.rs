use std::fmt::Debug;

use ethers::providers::{Http, JsonRpcClient, Ws};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serenity::async_trait;

#[derive(Debug)]
pub enum EvmJsonRpcClient {
    Http(Http),
    Ws(Ws),
}

#[async_trait]
impl JsonRpcClient for EvmJsonRpcClient {
    type Error = ethers::providers::ProviderError;

    async fn request<T, R>(&self, method: &str, params: T) -> Result<R, Self::Error>
    where
        T: Debug + Serialize + Send + Sync,
        R: DeserializeOwned + Send,
    {
        match self {
            EvmJsonRpcClient::Http(http) => Ok(http.request(method, params).await?),
            EvmJsonRpcClient::Ws(ws) => Ok(ws.request(method, params).await?),
        }
    }
}
