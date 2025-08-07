//! # Binance SOL Price Stream
//!
//! This module provides functionality to stream SOL/USDT price data from Binance's WebSocket API.
//!
//! ## Features
//!
//! - Real-time SOL price updates via WebSocket connection
//! - Fallback to REST API when WebSocket is unavailable
//! - Global price cache accessible throughout the application
//! - Optional integration with message queue for publishing price updates
//! - Optional integration with key-value store for persistence
//!
//! ## Usage
//!
//! ```rust
//! use sonar_sol_price::cpmm::{get_sol_price, SolPriceCache};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new price cache instance
//!     let price_cache = SolPriceCache::new(None, None);
//!
//!     // Start the price stream in a background task
//!     let price_cache_clone = price_cache.clone();
//!     tokio::spawn(async move {
//!         if let Err(e) = price_cache_clone.start_price_stream().await {
//!             eprintln!("Error in price stream: {}", e);
//!         }
//!     });
//!
//!     // Get the current SOL price
//!     let price = price_cache.get_price().await;
//!     println!("Current SOL price: ${:.3}", price);
//!
//!     // Or use the convenience function
//!     let global_price = get_sol_price().await;
//!     println!("Global SOL price: ${:.3}", global_price);
//! }
//! ```
//!
//! This implementation is based on the approach from
//! [piotrostr/listen](https://github.com/piotrostr/listen/blob/main/listen-data/src/sol_price_stream.rs)
//! with modifications to fit the sonar architecture.

use crate::{SolPriceCacheTrait, SOL_PRICE_CACHE};
use anyhow::Result;
use serde::Deserialize;
use sonar_db::{KvStore, MessageQueue};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

#[derive(Debug, Deserialize)]
struct BinancePrice {
    price: String,
}

#[derive(Clone)]
pub struct SolPriceCache {
    price: Arc<RwLock<f64>>,
    message_queue: Option<Arc<MessageQueue>>,
    kv_store: Option<Arc<KvStore>>,
}

impl SolPriceCache {
    pub fn new(kv_store: Option<Arc<KvStore>>, message_queue: Option<Arc<MessageQueue>>) -> Self {
        Self {
            price: SOL_PRICE_CACHE.clone(), // Use the global price cache
            message_queue,
            kv_store,
        }
    }

    pub async fn set_price(&self, price: f64) {
        *self.price.write().await = price;
    }

    pub async fn get_price(&self) -> f64 {
        let current_price = *self.price.read().await;
        if current_price == 0.0 {
            match self.fetch_rest_price().await {
                Ok(rest_price) => {
                    *self.price.write().await = rest_price;
                    rest_price
                }
                Err(e) => {
                    error!("Failed to fetch REST price: {}", e);
                    current_price
                }
            }
        } else {
            current_price
        }
    }

    async fn fetch_rest_price(&self) -> Result<f64> {
        let rest_url = "https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT";
        let response = reqwest::get(rest_url).await?;
        let price_data: BinancePrice = response.json().await?;
        price_data.price.parse::<f64>().map_err(Into::into)
    }

    pub async fn start_price_stream(&self) -> Result<()> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl SolPriceCacheTrait for SolPriceCache {
    fn get_name(&self) -> String {
        "cpmm".to_string()
    }

    fn get_owner(&self) -> String {
        "cpmm".to_string()
    }

    fn get_signature(&self) -> String {
        "cpmm_stream".to_string()
    }

    fn get_kv_store(&self) -> Option<Arc<KvStore>> {
        self.kv_store.clone()
    }

    fn get_message_queue(&self) -> Option<Arc<MessageQueue>> {
        self.message_queue.clone()
    }

    async fn set_price(&self, price: f64) -> Result<()> {
        *self.price.write().await = price;
        Ok(())
    }

    async fn get_price(&self) -> f64 {
        let current_price = *self.price.read().await;
        if current_price == 0.0 {
            match self.fetch_rest_price().await {
                Ok(price) => {
                    // Update the cache with the fetched price
                    let _ = self.set_price(price).await;
                    price
                }
                Err(e) => {
                    error!("Failed to fetch SOL price from REST API: {}", e);
                    0.0
                }
            }
        } else {
            current_price
        }
    }

    async fn start_price_stream(&self) -> Result<()> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    use tracing::{error, info};

    #[tokio::test]
    async fn test_sol_price_cache() {
        let price_cache = SolPriceCache::new(None, None);
        let price_cache_clone = price_cache.clone();

        // Spawn the price stream in a separate task
        tokio::spawn(async move {
            if let Err(e) = price_cache.start_price_stream().await {
                error!("Error in price stream: {}", e);
            }
        });

        // Wait a bit for the first price update
        sleep(Duration::from_secs(2)).await;

        let price = price_cache_clone.get_price().await;
        info!("Current SOL price: ${:.3}", price);
        assert!(price > 0.0, "Price should be greater than 0");
    }

    #[tokio::test]
    async fn test_rest_fallback() {
        let price_cache = SolPriceCache::new(None, None);

        // Test initial state (should trigger REST fallback)
        let price = price_cache.get_price().await;
        info!("Initial SOL price from REST: ${:.3}", price);
        assert!(price > 0.0, "REST fallback price should be greater than 0");

        // Test that the price was cached
        let cached_price = *price_cache.price.read().await;
        assert_eq!(price, cached_price, "Price should be cached after REST call");
    }
}
