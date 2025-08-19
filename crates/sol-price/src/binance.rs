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
use futures::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use sonar_db::{KvStore, MessageQueue, Trade};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::{
    connect_async, tungstenite::error::Error as WsError, tungstenite::protocol::Message,
    MaybeTlsStream, WebSocketStream,
};
use tracing::{error, info};
use url::Url;

// Type aliases to reduce complexity
type WebSocketStreamType = WebSocketStream<MaybeTlsStream<TcpStream>>;
type SplitSinkType = SplitSink<WebSocketStreamType, Message>;
type SplitStreamType = SplitStream<WebSocketStreamType>;

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

    /**
     * Publish the trade to the message queue and the KV store.
     *
     * @param new_price - The new price to publish.
     * @return Result<()> - The result of the operation.
     */
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

    /**
     * Set the price in the cache.
     *
     * @param price - The new price to set.
     */
    pub async fn set_price(&self, price: f64) {
        *self.price.write().await = price;
    }

    /**
     * Get the price from the cache.
     *
     * If the price is 0.0, fetch the price from the REST API.
     *
     * @return f64 - The current price.
     */
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

    /**
     * Fetch the price from the REST API.
     *
     * @return Result<f64> - The price.
     */
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

    /**
     * Connect to the Binance WebSocket and stream the price data.
     *
     * @return Result<()> - The result of the operation.
     */
    async fn connect_and_stream(&self) -> Result<()> {
        let url = Url::parse("wss://fstream.binance.com/ws/solusdt@aggTrade")?;
        let (ws_stream, _) = connect_async(url.to_string()).await?;
        info!("WebSocket connected to Binance SOL/USDT futures stream");

        let (write, mut read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));
        let heartbeat_handle = self.spawn_heartbeat_task(write.clone());

        let result = self.handle_message_stream(&mut read, &write).await;

        // abort the heartbeat task when the main loop exits
        heartbeat_handle.abort();
        result
    }

    /**
     * Spawn the heartbeat task.
     *
     * This is used to keep the connection alive.
     *
     * @param write - The write stream to the server.
     * @return tokio::task::JoinHandle<()> - The handle to the heartbeat task.
     */
    fn spawn_heartbeat_task(
        &self,
        write: Arc<Mutex<SplitSinkType>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;

                if let Err(e) = write.lock().await.send(Message::Pong(vec![].into())).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
        })
    }

    /**
     * Handle the message stream.
     *
     * This is used to handle the message stream from the server.
     *
     * @param read - The read stream from the server.
     * @param write - The write stream to the server.
     * @return Result<()> - The result of the operation.
     */
    async fn handle_message_stream(
        &self,
        read: &mut SplitStreamType,
        write: &Arc<Mutex<SplitSinkType>>,
    ) -> Result<()> {
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => self.handle_text_message(&text).await?,
                Ok(Message::Ping(_)) => self.handle_ping_message(write).await?,
                Ok(Message::Close(frame)) => self.handle_close_message(&frame).await?,
                Err(e) => self.handle_error_message(e).await?,
                _ => {}
            }
        }
        Ok(())
    }

    /**
     * Handle the text message from the server.
     *
     * This is used to handle the text message from the server.
     *
     * @param text - The text message from the server.
     * @return Result<()> - The result of the operation.
     */
    async fn handle_text_message(&self, text: &str) -> Result<()> {
        match serde_json::from_str::<TradeData>(text) {
            Ok(trade) => {
                if let Ok(new_price) = trade.p.parse::<f64>() {
                    self.process_price_update(new_price).await;
                } else {
                    error!("Failed to parse price: {}", text);
                }
            }
            Err(e) => error!("Error parsing JSON: {}", e),
        };
        Ok(())
    }

    /**
     * Handle the ping message from the server.
     *
     * This is used to keep the connection alive.
     *
     * @param write - The write stream to the server.
     * @return Result<()> - The result of the operation.
     */
    async fn handle_ping_message(&self, write: &Arc<Mutex<SplitSinkType>>) -> Result<()> {
        write
            .lock()
            .await
            .send(Message::Pong(vec![].into()))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send pong: {}", e))
    }

    /**
     * Handle the close message from the server.
     *
     * This is used to handle the close message from the server.
     *
     * @param frame - The close frame from the server.
     * @return Result<()> - The result of the operation.
     */
    async fn handle_close_message(&self, frame: &Option<CloseFrame>) -> Result<()> {
        info!("WebSocket closed by server: {:?}", frame);
        Ok(())
    }

    /**
     * Handle the  message from the server.
     *
     * This is used to keep the connection alive.
     *
     * @param write - The write stream to the server.
     * @return Result<()> - The result of the operation.
     */
    async fn handle_error_message(&self, error: WsError) -> Result<()> {
        error!("WebSocket error: {:?}", error);
        Err(anyhow::anyhow!("WebSocket error: {:?}", error))
    }

    /**
     * Process the price update.
     *
     * This is used to update the price cache and publish the trade.
     *
     * @param new_price - The new price to update the cache with.
     */
    async fn process_price_update(&self, new_price: f64) {
        let current_price = self.get_price().await;

        if current_price != new_price {
            self.set_price(new_price).await;
            let price_cache = self.clone();
            tokio::spawn(async move {
                if let Err(e) = price_cache.publish_trade(new_price).await {
                    error!("Failed to publish price update: {}", e);
                }
            });
        }
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
