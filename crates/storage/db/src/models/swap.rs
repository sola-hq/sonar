use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Swap {
    pub coin_price: f64,
    pub coin_mint: String,
    pub pc_mint: String,
    pub coin_decimals: u64,
    pub pc_decimals: u64,
}

#[derive(clickhouse::Row)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapEvent {
    pub pair: String,
    pub pubkey: String,
    pub price: f64,
    pub market_cap: f64,
    pub base_amount: f64,  // base amount
    pub quote_amount: f64, // quote amount
    pub swap_amount: f64,  // denoted as usd
    pub owner: String,
    pub signature: String,
    pub signers: Vec<String>,
    pub slot: u64,
    pub timestamp: u64,
    pub is_buy: bool,
    pub is_pump: bool,
}

impl SwapEvent {
    pub fn update_market_cap(&mut self, supply: f64) {
        self.market_cap = self.price * supply;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeQuery {
    pub tx_hash: String,
    pub address: Option<String>,
    pub token: Option<String>,
    pub pair: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(clickhouse::Row)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    #[serde(rename = "pair")]
    pub pair: String,
    #[serde(rename = "token")]
    pub pubkey: String,
    #[serde(rename = "price")]
    pub price: f64,
    #[serde(rename = "market_cap")]
    pub market_cap: f64,
    #[serde(rename = "base_amount")]
    pub base_amount: f64, // base amount
    #[serde(rename = "quote_amount")]
    pub quote_amount: f64, // quote amount
    #[serde(rename = "swap_amount")]
    pub swap_amount: f64, // denoted as usd
    #[serde(rename = "owner")]
    pub owner: String,
    #[serde(rename = "signature")]
    pub signature: String,
    #[serde(rename = "signers")]
    pub signers: Vec<String>,
    #[serde(rename = "slot")]
    pub slot: u64,
    #[serde(rename = "timestamp")]
    pub timestamp: u64,
    #[serde(rename = "is_buy")]
    pub is_buy: bool,
    #[serde(rename = "is_pump")]
    pub is_pump: bool,
}

impl From<SwapEvent> for Trade {
    fn from(swap_event: SwapEvent) -> Self {
        Trade {
            pair: swap_event.pair,
            pubkey: swap_event.pubkey,
            price: swap_event.price,
            market_cap: swap_event.market_cap,
            base_amount: swap_event.base_amount,
            quote_amount: swap_event.quote_amount,
            swap_amount: swap_event.swap_amount,
            owner: swap_event.owner,
            signature: swap_event.signature,
            signers: swap_event.signers,
            slot: swap_event.slot,
            timestamp: swap_event.timestamp,
            is_buy: swap_event.is_buy,
            is_pump: swap_event.is_pump,
        }
    }
}
