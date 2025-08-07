use crate::ws::event::ResponseEvent;
use anyhow::Result;
use futures::StreamExt;
use socketioxide::{adapter::Adapter, SocketIo};
use sonar_db::{RedisSubscriber, Trade};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::warn;

pub const CHANNEL_BUFFER_SIZE: usize = 4 * 1000; // 4k
pub struct IoProxy<A: Adapter> {
    io: Arc<SocketIo<A>>,
    redis_subscriber: Arc<RedisSubscriber>,
    pub channel_buffer_size: usize,
}

impl<A: Adapter> IoProxy<A> {
    pub fn new(
        redis_subscriber: Arc<RedisSubscriber>,
        io: Arc<SocketIo<A>>,
        channel_buffer_size: Option<usize>,
    ) -> Self {
        Self {
            redis_subscriber,
            io,
            channel_buffer_size: channel_buffer_size.unwrap_or(CHANNEL_BUFFER_SIZE),
        }
    }

    /// Set the channel buffer size for the trade receiver.
    #[allow(dead_code)]
    pub fn with_channel_buffer_size(mut self, channel_buffer_size: usize) -> Self {
        self.channel_buffer_size = channel_buffer_size;
        self
    }

    /// Spawn the redis subscriber and processor tasks.
    pub async fn spawn_handlers(&self) -> Result<()> {
        let redis_subscriber = self.redis_subscriber.clone();
        let channel_buffer_size = self.channel_buffer_size;
        let io = self.io.clone();

        let (trade_sender, trade_receiver) = mpsc::channel(channel_buffer_size);

        let redis_subscriber_clone = redis_subscriber.clone();
        let trade_sender_clone = trade_sender.clone();

        let trade_fetcher = trade_fetcher(redis_subscriber_clone, trade_sender_clone);
        let trade_processor = trade_processor(trade_receiver, io);

        tokio::spawn(async move {
            tokio::select! {
                _ = trade_fetcher  => {
                    warn!("Trade fetcher task completed");
                }
                _ = trade_processor => {
                    warn!("Trade processor task completed");
                }
            }
        });

        Ok(())
    }
}

/// Spawns a task to fetch trades from Redis and send them to the trade sender.
pub async fn trade_fetcher(redis_subscriber: Arc<RedisSubscriber>, trade_sender: Sender<Trade>) {
    let mut retry_count = 0;
    let channel_name = "trade";
    loop {
        match redis_subscriber.subscriber(channel_name).await {
            Ok(mut msg_stream) => {
                retry_count = 0; // Reset retry count on successful connection
                while let Some(msg) = msg_stream.next().await {
                    if let Ok(payload) = msg.get_payload::<String>() {
                        if let Ok(trade) = serde_json::from_str::<Trade>(&payload) {
                            if trade_sender.send(trade).await.is_err() {
                                warn!("Failed to send trade, retrying...");
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                retry_count += 1;
                warn!("Failed to subscribe to trade channel (attempt {}): {}", retry_count, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}

/// Process the task and send the trade to the sender
pub async fn trade_processor<A: Adapter>(trade_receiver: Receiver<Trade>, io: Arc<SocketIo<A>>) {
    let mut trade_receiver = trade_receiver;
    while let Some(trade) = trade_receiver.recv().await {
        if let Err(e) = io
            .to(trade.pubkey.to_string())
            .emit(ResponseEvent::TradeCreated.to_string(), &trade.clone())
            .await
        {
            warn!("Failed to emit trade to websocket: {}", e);
        }
    }
    warn!("Trade receiver channel closed");
}
