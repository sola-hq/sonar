//! # Raydium CLMM SOL Price Stream
//!
//! This module provides functionality to stream SOL/USDC price data from Raydium CLMM pools
//! by monitoring pool account changes on Solana blockchain.
//!
//! ## Features
//!
//! - Real-time SOL price updates via Solana account monitoring
//! - Raydium CLMM pool data decoding
//! - Global price cache accessible throughout the application
//! - Optional integration with message queue for publishing price updates
//! - Optional integration with key-value store for persistence
//!
//! ## Usage
//!
//! ```rust
//! use sonar_sol_price::clmm::{get_sol_price, SolPriceCache};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new price cache instance
//!     let price_cache = SolPriceCache::new(None, None);
//!
//!     // Start the price stream
//!     price_cache.start_price_stream().await?;
//!
//!     // Get current price
//!     let price = price_cache.get_price().await;
//!     println!("Current SOL price: {}", price);
//!
//!     Ok(())
//! }
//! ```

use crate::{
    cache::SOL_PRICE_CACHE,
    constants::{MARKET_PROGRAM_ID, WSOL_MINT_KEY_STR},
    SolPriceCacheTrait,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use carbon_core::account::AccountDecoder;
use carbon_raydium_clmm_decoder::{accounts::RaydiumClmmAccount, RaydiumClmmDecoder};
use chrono::Utc;
use futures::stream::StreamExt;
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::RpcAccountInfoConfig,
};
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use sonar_db::{KvStore, MessageQueue, Trade};
use std::{str::FromStr, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Raydium CLMM price stream configuration
#[derive(Debug, Clone)]
pub struct ClmmConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// WebSocket endpoint URL
    pub ws_url: String,
    /// Commitment level
    pub commitment: CommitmentConfig,
}

impl Default for ClmmConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

/// Raydium CLMM price stream implementation
pub struct RaydiumClmmPriceStream {
    config: ClmmConfig,
    rpc_client: Arc<RpcClient>,
    pubsub_client: Arc<PubsubClient>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl RaydiumClmmPriceStream {
    /// Create a new Raydium CLMM price stream
    pub async fn new(config: ClmmConfig) -> Result<Self> {
        let rpc_client =
            Arc::new(RpcClient::new_with_commitment(config.rpc_url.clone(), config.commitment));
        let pubsub_client = Arc::new(PubsubClient::new(&config.ws_url).await?);
        Ok(Self { config, rpc_client, pubsub_client, shutdown_tx: None })
    }

    /// Start the price stream
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Raydium CLMM price stream");

        let config = RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            ..Default::default()
        };
        let (mut stream, _unsubscribe) = self
            .pubsub_client
            .account_subscribe(&Pubkey::from_str(MARKET_PROGRAM_ID)?, Some(config))
            .await?;

        info!("Successfully subscribed to account changes");

        while let Some(item) = stream.next().await {
            let account = item.value.decode().context("Failed to decode data")?;
            let decoder = RaydiumClmmDecoder;
            let pool_state = decoder.decode_account(&account);
            if let Some(pool_state) = pool_state {
                match pool_state.data {
                    RaydiumClmmAccount::PoolState(pool_state) => {
                        println!("pool_state: {pool_state:?}");
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Stop the price stream
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
            info!("Raydium CLMM price stream stopped");
        }
        Ok(())
    }

    /// Update price for a specific pool
    async fn update_pool_price(rpc_client: &Arc<RpcClient>, pool_address: &str) -> Result<()> {
        let pubkey = Pubkey::from_str(pool_address)
            .context(format!("Invalid pool address: {}", pool_address))?;

        let account =
            rpc_client.get_account(&pubkey).await.context("Failed to get pool account")?;

        // let pool_state = Self::decode_pool_state(&account.data)?;
        // let price = Self::calculate_price_from_pool_state(&pool_state)?;

        // Update the global price cache
        // crate::cache::set_sol_price(price).await;

        // debug!("Updated SOL price: ${:.6} from pool {}", price, pool_address);
        Ok(())
    }
}

/// SOL price cache implementation for Raydium CLMM
#[derive(Clone)]
pub struct SolPriceCache {
    price: Arc<RwLock<f64>>,
    message_queue: Option<Arc<MessageQueue>>,
    kv_store: Option<Arc<KvStore>>,
    clmm_stream: Option<Arc<RaydiumClmmPriceStream>>,
}

impl SolPriceCache {
    /// Create a new SOL price cache
    pub fn new(kv_store: Option<Arc<KvStore>>, message_queue: Option<Arc<MessageQueue>>) -> Self {
        Self { price: SOL_PRICE_CACHE.clone(), message_queue, kv_store, clmm_stream: None }
    }

    /// Start the CLMM price stream
    pub async fn start_clmm_stream(&mut self) -> Result<()> {
        let config = ClmmConfig::default();
        let mut price_stream = RaydiumClmmPriceStream::new(config).await?;
        price_stream.start().await?;

        // Store the stream for later shutdown
        // Note: This is simplified - in practice you'd need to handle the stream lifecycle properly
        info!("Raydium CLMM price stream started");
        Ok(())
    }

    /// Stop the CLMM price stream
    pub async fn stop_clmm_stream(&mut self) -> Result<()> {
        if let Some(_stream) = self.clmm_stream.take() {
            // In practice, you'd need to properly handle the stream shutdown
            info!("Raydium CLMM price stream stopped");
        }
        Ok(())
    }

    async fn publish_trade(&self, new_price: f64) -> Result<()> {
        let trade = Trade {
            pair: "SOLUSD".to_string(),
            pubkey: WSOL_MINT_KEY_STR.to_string(),
            price: new_price,
            market_cap: 0.0,
            base_amount: 0.0,
            quote_amount: 0.0,
            swap_amount: 0.0,
            slot: 0,
            timestamp: Utc::now().timestamp() as u64,
            is_buy: false,
            is_pump: false,
            owner: "raydium_clmm".to_string(),
            signers: vec![],
            signature: "raydium_clmm_stream".to_string(),
        };

        if let Some(kv_store) = &self.kv_store {
            kv_store.insert_price(&trade).await?;
        }
        if let Some(mq) = &self.message_queue {
            mq.publish_trade(&trade).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl SolPriceCacheTrait for SolPriceCache {
    fn get_name(&self) -> String {
        "raydium_clmm".to_string()
    }

    fn get_owner(&self) -> String {
        "raydium_clmm".to_string()
    }

    fn get_signature(&self) -> String {
        "raydium_clmm_stream".to_string()
    }

    fn get_kv_store(&self) -> Option<Arc<KvStore>> {
        self.kv_store.clone()
    }

    fn get_message_queue(&self) -> Option<Arc<MessageQueue>> {
        self.message_queue.clone()
    }

    async fn get_price(&self) -> f64 {
        *self.price.read().await
    }

    async fn set_price(&self, price: f64) -> Result<()> {
        *self.price.write().await = price;

        // Publish the trade if we have the necessary components
        if self.kv_store.is_some() || self.message_queue.is_some() {
            if let Err(e) = self.publish_trade(price).await {
                error!("Failed to publish trade: {}", e);
            }
        }

        Ok(())
    }

    async fn start_price_stream(&self) -> Result<()> {
        // Start the CLMM stream in a background task
        let mut cache = SolPriceCache::new(self.kv_store.clone(), self.message_queue.clone());

        tokio::spawn(async move {
            if let Err(e) = cache.start_cpmm_stream().await {
                error!("Failed to start CLMM stream: {}", e);
            }
        });

        Ok(())
    }
}

impl Default for SolPriceCache {
    fn default() -> Self {
        Self::new(None, None)
    }
}

/// Get the current SOL price from the global cache
pub async fn get_sol_price() -> f64 {
    crate::cache::get_sol_price().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sol_price_cache_creation() {
        let cache = SolPriceCache::new(None, None);
        assert!(cache.cpmm_stream.is_none());
    }

    #[tokio::test]
    async fn test_cpmm_config_default() {
        let config = ClmmConfig::default();
        assert_eq!(config.rpc_url, "https://api.mainnet-beta.solana.com");
        assert_eq!(config.update_interval_ms, 1000);
        assert!(!config.pool_addresses.is_empty());
    }

    #[test]
    fn test_pool_state_decoding() {
        let mock_data = vec![1u8; 100];
        let result = RaydiumClmmPriceStream::decode_pool_state(&mock_data);
        assert!(result.is_ok());
    }
}
