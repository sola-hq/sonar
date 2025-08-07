use serde::{Deserialize, Serialize};

#[derive(clickhouse::Row)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TopToken {
    pub pubkey: String,
    pub price: f64,
    pub market_cap: f64,
    pub volume: f64,
    pub turnover: f64,
    pub price_change: f64,
}

#[derive(clickhouse::Row)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStat {
    pub pubkey: String,
    pub price: f64,
    pub market_cap: f64,
    pub price_5m: f64,
    pub price_1h: f64,
    pub price_6h: f64,
    pub price_24h: f64,
    pub volume_5m: f64,
    pub volume_1h: f64,
    pub volume_6h: f64,
    pub volume_24h: f64,
    pub turnover_5m: f64,
    pub turnover_1h: f64,
    pub turnover_6h: f64,
    pub turnover_24h: f64,
}

#[derive(clickhouse::Row)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDailyStat {
    pub pubkey: String,
    pub timestamp: u64,
    pub price: f64,
    pub market_cap: f64,
    pub price_24h: f64,
    pub volume_24h: f64,
    pub turnover_24h: f64,
}

#[derive(clickhouse::Row)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub token: String,
    pub timestamp: i32,
    pub price: Option<f64>,
    pub neatest_timestamp: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub address: String,
    pub verified: bool,
    pub share: u8,
}

impl From<mpl_token_metadata::types::Creator> for Creator {
    fn from(creator: mpl_token_metadata::types::Creator) -> Self {
        Self {
            address: creator.address.to_string(),
            verified: creator.verified,
            share: creator.share,
        }
    }
}

/// Cleans a metadata string by removing null terminators
///
/// # Arguments
///
/// * `s`: The metadata string to clean
///
/// # Returns
///
/// * `String`: The cleaned metadata string
///
/// # Examples
///
/// ```rust
/// use sonar_db::clean_string;
/// let s = "USD Coin\0\0\0";
/// let cleaned = clean_string(s);
/// assert_eq!(cleaned, "USD Coin");
/// ```
pub fn clean_string(s: &str) -> String {
    s.trim_matches(char::from(0)).to_string()
}

#[derive(clickhouse::Row)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub retrieval_timestamp: u64,
    pub is_nft: bool,
    pub token: String,
    pub update_authority: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub supply: f64,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
}

#[derive(clickhouse::Row)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSearch {
    pub token: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub supply: f64,
    pub latest_price: f64,
    pub price_24h: f64,
    pub tx_count_24h: u64,
    pub volume_24h: f64,
    pub turnover_24h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub mint: String,
    pub update_authority: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
}

impl From<mpl_token_metadata::accounts::Metadata> for TokenMetadata {
    fn from(metadata: mpl_token_metadata::accounts::Metadata) -> Self {
        Self {
            mint: metadata.mint.to_string(),
            update_authority: metadata.update_authority.to_string(),
            name: clean_string(&metadata.name),
            symbol: clean_string(&metadata.symbol),
            uri: clean_string(&metadata.uri),
            seller_fee_basis_points: metadata.seller_fee_basis_points,
            primary_sale_happened: metadata.primary_sale_happened,
            is_mutable: metadata.is_mutable,
        }
    }
}

impl From<spl_token_metadata_interface::state::TokenMetadata> for TokenMetadata {
    fn from(metadata: spl_token_metadata_interface::state::TokenMetadata) -> Self {
        Self {
            mint: metadata.mint.to_string(),
            update_authority: metadata.update_authority.0.to_string(),
            name: clean_string(&metadata.name),
            symbol: clean_string(&metadata.symbol),
            uri: clean_string(&metadata.uri),
            seller_fee_basis_points: 0,
            primary_sale_happened: false,
            is_mutable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_metadata_string() {
        let usdc_name = "USD Coin\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(clean_string(usdc_name), "USD Coin");

        let usdc_symbol = "USDC\0\0\0\0\0\0";
        assert_eq!(clean_string(usdc_symbol), "USDC");

        let usdc_uri = "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(clean_string(usdc_uri), "");
    }
}
