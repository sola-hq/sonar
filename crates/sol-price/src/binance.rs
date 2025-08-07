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
//! use sonar_sol_price::binance_stream::{get_sol_price, SolPriceCache};
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
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use sonar_db::{KvStore, MessageQueue, Trade};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info};
use url::Url;

#[derive(Debug, Deserialize)]
struct TradeData {
    p: String,
}

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

    async fn publish_trade(&self, new_price: f64) -> Result<()> {
        let trade: Trade = Trade {
            pair: "SOLUSD".to_string(),
            pubkey: crate::constants::WSOL_MINT_KEY_STR.to_string(),
            price: new_price,
            market_cap: 0.0,
            base_amount: 0.0,
            quote_amount: 0.0,
            swap_amount: 0.0,
            slot: 0,
            timestamp: Utc::now().timestamp() as u64,
            is_buy: false,
            is_pump: false,
            owner: "binance".to_string(),
            signers: vec![],
            signature: "binance_websocket".to_string(),
        };
        if let Some(kv_store) = &self.kv_store {
            kv_store.insert_price(&trade).await?;
        }
        if let Some(mq) = &self.message_queue {
            mq.publish_trade(&trade).await?;
        }
        Ok(())
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
        loop {
            info!("Connecting to Binance WebSocket...");
            match self.connect_and_stream().await {
                Ok(_) => {
                    info!("WebSocket stream ended gracefully");
                    // Optional delay before reconnecting on graceful close
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                Err(e) => {
                    error!("WebSocket stream error: {}. Reconnecting in 5 seconds...", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn connect_and_stream(&self) -> Result<()> {
        let url = Url::parse("wss://fstream.binance.com/ws/solusdt@aggTrade")?;
        let (ws_stream, _) = connect_async(url).await?;
        let price_cache = self.clone();
        info!("WebSocket connected to Binance SOL/USDT futures stream");

        let (write, mut read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));
        let write_clone = write.clone();

        // Spawn a task to handle pong responses
        let pong_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                if let Err(e) = write_clone.lock().await.send(Message::Pong(vec![])).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
        });

        let result = async {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => match serde_json::from_str::<TradeData>(&text) {
                        Ok(trade) => {
                            if let Ok(new_price) = trade.p.parse::<f64>() {
                                let current_price = price_cache.get_price().await;
                                if current_price != new_price {
                                    price_cache.set_price(new_price).await;
                                    let price_cache = price_cache.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = price_cache.publish_trade(new_price).await {
                                            error!("Failed to publish price update: {}", e);
                                        }
                                    });
                                }
                            } else {
                                error!("Failed to parse price: {}", text);
                            }
                        }
                        Err(e) => error!("Error parsing JSON: {}", e),
                    },
                    Ok(Message::Ping(_)) => {
                        if let Err(e) = write.lock().await.send(Message::Pong(vec![])).await {
                            error!("Failed to send pong: {}", e);
                            return Err(anyhow::anyhow!("Pong send error: {}", e));
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        info!("WebSocket closed by server: {:?}", frame);
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("WebSocket error: {}", e));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        .await;

        // Cancel the pong task when the main loop exits
        pong_task.abort();
        result
    }
}

#[async_trait::async_trait]
impl SolPriceCacheTrait for SolPriceCache {
    fn get_name(&self) -> String {
        "binance".to_string()
    }

    fn get_owner(&self) -> String {
        "binance".to_string()
    }

    fn get_signature(&self) -> String {
        "binance_websocket".to_string()
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
        self.connect_and_stream().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::get_sol_price;
    use tokio::time::{sleep, Duration};
    use tracing_otel_extra::init_logging;

    #[tokio::test]
    async fn test_sol_price_cache() {
        init_logging("binance_sol_price").expect("Failed to initialize logging");
        let price_cache = SolPriceCache::new(None, None);
        let price_cache_clone = price_cache.clone();

        // Spawn the price stream in a separate task
        tokio::spawn(async move {
            if let Err(e) = price_cache.start_price_stream().await {
                error!("Error in price stream: {}", e);
            }
        });

        // Wait a bit for the first price update
        sleep(Duration::from_secs(10)).await;

        let price = price_cache_clone.get_price().await;
        info!("Current SOL price: ${:.3}", price);
        assert!(price > 0.0, "Price should be greater than 0");

        let price = get_sol_price().await;
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
