use crate::ws::event::{RequestEvent, TokenHolderEvent};
use socketioxide::{adapter::Adapter, SocketIo};
use std::sync::Arc;

pub const CHANNEL_BUFFER_SIZE: usize = 4 * 1000; // 4k

#[derive(Clone)]
pub struct IoProxy<A: Adapter> {
    io: Arc<SocketIo<A>>,
    pub channel_buffer_size: usize,
}

impl<A: Adapter> IoProxy<A> {
    pub fn new(io: Arc<SocketIo<A>>, channel_buffer_size: Option<usize>) -> Self {
        Self { io, channel_buffer_size: channel_buffer_size.unwrap_or(CHANNEL_BUFFER_SIZE) }
    }

    /// Set the channel buffer size for the trade receiver.
    #[allow(dead_code)]
    pub fn with_channel_buffer_size(&mut self, channel_buffer_size: usize) -> &mut Self {
        self.channel_buffer_size = channel_buffer_size;
        self
    }

    pub async fn broadcast_token_holder(&self, data: &TokenHolderEvent) {
        self.io
            .emit(RequestEvent::TokenHolder.to_string(), data)
            .await
            .expect("Failed to emit token_holder broadcast");
    }
}
