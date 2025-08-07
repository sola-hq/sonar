use crate::models::{swap::Trade, Token};
use anyhow::{Context, Result};
use bb8_redis::{bb8, redis::AsyncCommands, RedisConnectionManager};
use serde::{de::DeserializeOwned, Serialize};
use std::env::var;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct KvStore {
    pool: bb8::Pool<RedisConnectionManager>,
}

impl KvStore {
    pub(crate) async fn get_connection(
        &self,
    ) -> Result<bb8::PooledConnection<'_, RedisConnectionManager>> {
        let conn = self.pool.get().await.context(format!(
            "Failed to get Redis connection: {:#?}",
            self.pool.state().statistics
        ))?;
        Ok(conn)
    }

    pub async fn new(redis_url: &str) -> Result<Self> {
        let pool = make_kv_pool(redis_url).await?;
        info!("Connected to Redis KV store at {}", redis_url);
        Ok(Self { pool })
    }

    pub async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.get_connection().await?;

        let value: Option<String> =
            conn.get(key).await.context(format!("Failed to get value for key: {}", key))?;

        value
            .map(|json_str| {
                serde_json::from_str(&json_str)
                    .with_context(|| format!("Failed to deserialize value for key: {}", key))
            })
            .transpose()
    }

    pub async fn set_ex<T: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: &T,
        seconds: u64,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let json_str = serde_json::to_string(value)?;
        let _: () = conn
            .set_ex(key, json_str, seconds)
            .await
            .context(format!("Failed to set key: {}", key))?;
        debug!(key, "redis set ok");
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let exists: bool =
            conn.exists(key).await.context(format!("Failed to check if key exists: {}", key))?;
        debug!(key, exists, "redis exists ok");
        Ok(exists)
    }

    fn get_price_key(&self, mint: &str) -> String {
        format!("solana:price:{}", mint)
    }

    fn get_price_history_key(&self, mint: &str) -> String {
        format!("solana:price:history:{}", mint)
    }

    pub async fn insert_price(&self, price: &Trade) -> Result<()> {
        let key = self.get_price_key(&price.pubkey);
        self.set_ex(&key, price, 60 * 60 * 24).await
    }

    pub async fn get_price(&self, mint: &str) -> Result<Option<Trade>> {
        let key = self.get_price_key(mint);
        self.get(&key).await
    }

    // use zset to store price at timestamp
    pub async fn set_price_at_timestamp(
        &self,
        mint: &str,
        price: f64,
        timestamp: u64,
    ) -> Result<()> {
        let key = self.get_price_history_key(mint);
        let mut conn = self.get_connection().await?;
        conn.zadd::<_, _, _, ()>(key, price, timestamp)
            .await
            .context(format!("Failed to set price at timestamp: {}", timestamp))?;
        Ok(())
    }

    pub async fn get_price_at_timestamp(&self, mint: &str, timestamp: u64) -> Result<f64> {
        let key = self.get_price_history_key(mint);
        let mut conn = self.get_connection().await?;
        let price: Vec<f64> = conn
            .zrevrangebyscore_limit(key, timestamp, 0, 0, 1)
            .await
            .context(format!("Failed to get price at timestamp: {}", timestamp))?;
        let price = price.first().copied().unwrap_or(0.0);
        Ok(price)
    }

    fn get_token_key(&self, pubkey: &str) -> String {
        format!("solana:metadata:{}", pubkey)
    }

    pub async fn set_token(&self, mint: &str, token: &Token) -> Result<()> {
        let key = self.get_token_key(mint);
        self.set_ex(&key, token, 60 * 60 * 24).await
    }

    pub async fn get_token(&self, mint: &str) -> Result<Option<Token>> {
        let key = self.get_token_key(mint);
        self.get(&key).await
    }

    pub async fn has_token(&self, mint: &str) -> Result<bool> {
        let key = self.get_token_key(mint);
        self.exists(&key).await
    }
}

pub async fn make_kv_store(redis_url: &str) -> Result<KvStore> {
    let kv = KvStore::new(redis_url).await?;
    Ok(kv)
}

pub async fn make_kv_store_from_env() -> Result<KvStore> {
    let redis_url = var("REDIS_URL").expect("Expected REDIS_URL to be set");
    make_kv_store(&redis_url).await
}

/// make a redis connection pool
/// https://github.com/djc/bb8
pub async fn make_kv_pool(redis_url: &str) -> Result<bb8::Pool<RedisConnectionManager>> {
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = bb8::Pool::builder()
        .max_size(200)
        .min_idle(Some(20))
        .max_lifetime(Some(std::time::Duration::from_secs(60 * 15))) // 15 minutes
        .idle_timeout(Some(std::time::Duration::from_secs(60 * 5))) // 5 minutes
        .build(manager)
        .await?;
    Ok(pool)
}
