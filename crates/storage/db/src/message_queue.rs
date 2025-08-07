use crate::{kv_store::make_kv_pool, models::swap::Trade};
use anyhow::{Context, Result};
use bb8_redis::{bb8, RedisConnectionManager};
use std::env::var;
use tracing::info;

/// A boxed message queue
pub type MessageQueue = Box<dyn MessageQueueTrait + Send + Sync>;

#[async_trait::async_trait]
pub trait MessageQueueTrait {
    async fn new(url: &str) -> Result<Self>
    where
        Self: Sized;

    async fn publish_trade(&self, trade: &Trade) -> Result<()>;
}

// Redis implementation of MessageQueue
#[derive(Debug, Clone)]
pub struct RedisMessageQueue {
    pool: bb8::Pool<RedisConnectionManager>,
}

impl RedisMessageQueue {
    async fn publish_message(&self, channel: &str, payload: &str) -> Result<()> {
        let mut conn = self.pool.get().await.context(format!(
            "Failed to get Redis connection: {:#?}",
            self.pool.state().statistics
        ))?;
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(payload)
            .query_async::<()>(&mut *conn)
            .await
            .context("Failed to publish to Redis")?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl MessageQueueTrait for RedisMessageQueue {
    async fn new(url: &str) -> Result<Self> {
        let pool = make_kv_pool(url).await?;
        info!("Connected to Redis message queue at {}", url);
        Ok(Self { pool })
    }

    async fn publish_trade(&self, price_update: &Trade) -> Result<()> {
        let payload =
            serde_json::to_string(price_update).context("Failed to serialize price update")?;
        let channel = "trade";
        self.publish_message(channel, &payload).await?;

        Ok(())
    }
}

pub async fn make_message_queue(redis_url: &str) -> Result<MessageQueue> {
    let message_queue = RedisMessageQueue::new(redis_url).await?;
    Ok(Box::new(message_queue))
}

pub async fn make_message_queue_from_env() -> Result<MessageQueue> {
    let redis_url = var("REDIS_URL").expect("Expected REDIS_URL to be set");
    make_message_queue(&redis_url).await
}
