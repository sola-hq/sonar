use crate::ws::event::{LpEvent, RequestEvent, TokenHolderEvent};
use carbon_core::account::AccountMetadata;
use serde_json::{json, Value};
use socketioxide::{adapter::Adapter, BroadcastError, SocketIo};
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

    pub async fn broadcast_account_change(
        &self,
        owner: &solana_pubkey::Pubkey,
        meta: AccountMetadata,
        data: Value,
    ) -> Result<(), BroadcastError> {
        let data = json!({
            "owner": owner.to_string(),
            "pubkey": meta.pubkey.to_string(),
            "data": data
        });

        self.io.to(owner.to_string()).emit(RequestEvent::AccountChange.to_string(), &data).await?;
        Ok(())
    }

    pub async fn broadcast_token_holder(
        &self,
        data: &TokenHolderEvent,
    ) -> Result<(), BroadcastError> {
        self.io.emit(RequestEvent::TokenHolder.to_string(), data).await?;
        Ok(())
    }

    pub async fn broadcast_lp(&self, data: &LpEvent) -> Result<(), BroadcastError> {
        self.io.emit(RequestEvent::Lp.to_string(), data).await?;
        Ok(())
    }
}
