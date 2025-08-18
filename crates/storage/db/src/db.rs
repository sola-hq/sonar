use crate::models::{
    candlesticks::{Candlestick, CandlestickInterval},
    swap::{SwapEvent, Trade},
    tokens::{Token, TokenDailyStat, TokenPrice, TokenSearch, TokenStat, TopToken},
};
use anyhow::Result;

/// A boxed database
pub type Database = Box<dyn DatabaseTrait + Send + Sync>;

#[async_trait::async_trait]
pub trait DatabaseTrait {
    fn new(database_url: &str, password: &str, user: &str, database: &str) -> Self
    where
        Self: Sized;
    async fn initialize(&mut self) -> Result<()>;
    async fn health_check(&self) -> Result<()>;

    /// uses a batched writer to avoid spamming writes
    async fn insert_swap_event(&self, swap_event: &SwapEvent) -> Result<()>;

    /// returns a list of candlesticks for a given token and interval
    async fn get_candlesticks_by_token(
        &self,
        token: &str,
        pairs: &[String],
        interval: CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
    ) -> Result<Vec<Candlestick>>;

    /// returns a list of candlesticks for a given pair and interval
    async fn get_candlesticks_by_pair(
        &self,
        pair: &str,
        token: Option<&str>,
        interval: &CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
    ) -> Result<Vec<Candlestick>>;

    /// returns a list of candlesticks for a given pair and interval
    async fn get_candlesticks_from_swap_events(
        &self,
        pair: &str,
        token: Option<&str>,
        interval: &CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
    ) -> Result<Vec<Candlestick>>;

    /// returns a list of candlesticks for a given pair and interval
    #[allow(clippy::too_many_arguments)]
    async fn get_candlesticks_from_candlesticks(
        &self,
        pair: &str,
        token: Option<&str>,
        interval: &CandlestickInterval,
        limit: Option<usize>,
        time_from: Option<i32>,
        time_to: Option<i32>,
        exclude_buckets: Option<Vec<u64>>,
    ) -> Result<Vec<Candlestick>>;

    /// returns a list of top tokens for a given
    /// limit
    /// min_volume
    /// min_market_cap
    /// time_range
    /// and pumpfun
    async fn get_top_tokens(
        &self,
        limit: usize,
        start_time: u64,
        min_volume: Option<f64>,
        min_market_cap: Option<f64>,
        pumpfun: Option<bool>,
    ) -> Result<Vec<TopToken>>;

    /// returns a list of token stats for a given list of tokens
    async fn get_token_stats(&self, tokens: Vec<String>) -> Result<Vec<TokenStat>>;

    /// returns a list of token daily stats for a given list of tokens
    async fn get_token_daily_stats(&self, tokens: Vec<String>) -> Result<Vec<TokenDailyStat>>;

    /// returns a list of swap events for a given query
    async fn get_trades(
        &self,
        address: Option<&str>,
        token: Option<&str>,
        pair: Option<&str>,
        signature: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Trade>>;

    /// get_price returns the price of a given mint at a given timestamp
    async fn get_price(&self, mint: &str, timestamp: i32) -> Result<TokenPrice>;

    /// get_prices returns prices for multiple mint and timestamp combinations
    async fn get_prices(&self, queries: Vec<(&str, i32)>) -> Result<Vec<TokenPrice>>;

    /// insert_token inserts a token into the database
    async fn insert_token(&self, token: &Token) -> Result<()>;

    /// get_token returns a token from the database
    async fn get_token(&self, mint: &str) -> Result<Option<Token>>;

    /// get_tokens returns tokens from the database
    async fn get_tokens(&self, mints: &[&str]) -> Result<Vec<Token>>;

    /// has_token returns true if a token exists in the database
    async fn has_token(&self, mint: &str) -> Result<bool>;

    /// search_tokens returns a list of tokens that match a given query
    async fn search_tokens(&self, query: &str) -> Result<Vec<TokenSearch>>;

    /// aggregates swap events into candlesticks table
    async fn aggregate_into_candlesticks(
        &self,
        start_time: i64,
        end_time: i64,
        interval: CandlestickInterval,
    ) -> Result<()>;

    /// remove_swap_events removes swap events from the database
    async fn remove_swap_events(&self, partition: i64) -> Result<()>;
}
