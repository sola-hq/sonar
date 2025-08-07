use anyhow::Result;
use async_stream::stream;
use futures::{Stream, StreamExt};
use redis::{AsyncCommands, Msg};
use std::{env, pin::Pin};
use tracing::info;

#[derive(Clone)]
pub struct RedisSubscriber {
    client: redis::Client,
}

impl RedisSubscriber {
    /// Create a new RedisSubscriber
    ///
    /// # Arguments
    ///
    /// * `redis_url` - The URL of the Redis server
    ///
    /// # Returns
    ///
    /// A new RedisSubscriber
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    /// Publish a message to a channel
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to publish the message to
    /// * `message` - The message to publish
    ///
    pub async fn publish(&self, channel: &str, message: &str) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.publish(channel, message).await?;
        Ok(())
    }

    /// Subscribe to a channel
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to subscribe to
    ///
    /// # Returns
    pub async fn subscriber(
        &self,
        channel: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Msg> + Send>>> {
        let mut pubsub_conn = self.client.get_async_pubsub().await?;
        info!("Subscribing to Redis channel: {}", channel);
        let _: () = pubsub_conn.subscribe(channel).await?;

        let stream = stream! {
            while let Some(msg) = pubsub_conn.on_message().next().await {
                yield msg;
            }
        };

        Ok(Box::pin(stream))
    }
}

pub async fn make_redis_subscriber(redis_url: &str) -> Result<RedisSubscriber> {
    let subscriber = RedisSubscriber::new(redis_url)?;
    Ok(subscriber)
}

pub async fn make_redis_subscriber_from_env() -> Result<RedisSubscriber> {
    let redis_url = env::var("REDIS_URL").expect("Expected REDIS_URL to be set");
    make_redis_subscriber(&redis_url).await
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_redis_subscriber() {
        let channel = "trade";
        let channel_msg = "testing message";
        let subscriber = make_redis_subscriber("redis://localhost:6379").await.unwrap();

        let mut stream =
            subscriber.subscriber(channel).await.expect("Failed to subscribe to channel");

        let subscriber_clone = subscriber.clone();
        tokio::spawn(async move {
            subscriber_clone.publish(channel, channel_msg).await.unwrap();
        });

        if let Some(msg) = stream.next().await {
            let payload: String = msg.get_payload().unwrap();
            assert_eq!(payload, channel_msg);
        } else {
            panic!("No message received");
        }
    }
}
