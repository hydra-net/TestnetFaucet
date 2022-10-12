use web3::types::U256;

pub fn wei_to_eth(amount: U256, decimals: u32) -> f64 {
    amount.as_u128() as f64 / 10_f64.powf(decimals as f64)
}

pub fn eth_to_wei(amount: f64, decimals: u32) -> U256 {
    if amount >= 10_f64.powf(-1.0 * decimals as f64) {
        let res = amount * 10_u64.pow(decimals) as f64;
        U256::from_dec_str(&res.to_string()).unwrap()
    } else {
        U256::from_dec_str("0").unwrap()
    }
}

pub fn sat_to_btc(amount: i64, decimals: u32) -> f64 {
    let res = amount as f64 / 10_f64.powf(decimals as f64);
    if res >= 0.0 {
        res
    } else {
        res * -1.0
    }
}

pub fn btc_to_sat(amount: f64, decimals: u32) -> u64 {
    if amount >= 10_f64.powf(-1.0 * decimals as f64) {
        let res = amount * 10_u64.pow(decimals) as f64;
        res as u64
    } else {
        0
    }
}
