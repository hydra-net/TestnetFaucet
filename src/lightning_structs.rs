use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SendCoinsRequest {
    /// The address to send coins to
    pub addr: String,
    /// The amount in satoshis to send
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub amount: i64,
    /// The target number of blocks that this transaction should be confirmed
    /// by.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub target_conf: i32,
    /// A manual fee rate set in sat/vbyte that should be used when crafting the
    /// transaction.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub sat_per_vbyte: u64,
    /// Deprecated, use sat_per_vbyte.
    /// A manual fee rate set in sat/vbyte that should be used when crafting the
    /// transaction.
    #[deprecated]
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub sat_per_byte: i64,
    ///
    ///If set, then the amount field will be ignored, and lnd will attempt to
    ///send all the coins under control of the internal wallet to the specified
    ///address.
    #[serde(deserialize_with = "deserialize_bool_from_anything")]
    pub send_all: bool,
    /// An optional label for the transaction, limited to 500 characters.
    pub label: String,
    /// The minimum number of confirmations each one of your outputs used for
    /// the transaction must satisfy.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub min_confs: i32,
    /// Whether unconfirmed outputs should be used as inputs for the transaction.
    #[serde(deserialize_with = "deserialize_bool_from_anything")]
    pub spend_unconfirmed: bool,
}
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SendCoinsResponse {
    /// The transaction ID of the transaction
    pub txid: String,
}