use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use sonar_db::{KvStore, MessageQueue, Trade};
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;

// Change the global cache to be just the price without Redis connections
pub static SOL_PRICE_CACHE: LazyLock<Arc<RwLock<f64>>> =
    LazyLock::new(|| Arc::new(RwLock::new(0.0)));

// Add a convenience function for getting the global price
pub async fn get_sol_price() -> f64 {
    *SOL_PRICE_CACHE.read().await
}

pub async fn set_sol_price(price: f64) {
    *SOL_PRICE_CACHE.write().await = price;
}

#[async_trait]
pub trait SolPriceCacheTrait {
    fn get_name(&self) -> String;
    fn get_owner(&self) -> String;
    fn get_signature(&self) -> String;

    fn get_kv_store(&self) -> Option<Arc<KvStore>>;
    fn get_message_queue(&self) -> Option<Arc<MessageQueue>>;

    async fn get_price(&self) -> f64;
    async fn set_price(&self, price: f64) -> Result<()>;
    async fn start_price_stream(&self) -> Result<()>;

    async fn publish_trade(&self, new_price: f64) -> Result<()> {
        let trade = Trade {
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
            owner: self.get_owner(),
            signers: vec![],
            signature: self.get_signature(),
        };
        if let Some(kv_store) = &self.get_kv_store() {
            kv_store.insert_price(&trade).await?;
        }
        if let Some(mq) = &self.get_message_queue() {
            mq.publish_trade(&trade).await?;
        }
        Ok(())
    }

    async fn get_price_at_timestamp(&self, timestamp: u64) -> Option<f64> {
        if let Some(kv_store) = &self.get_kv_store() {
            if let Ok(price) =
                kv_store.get_price_at_timestamp(crate::constants::SOLANNA, timestamp).await
            {
                return Some(price);
            }
        }
        None
    }
}
