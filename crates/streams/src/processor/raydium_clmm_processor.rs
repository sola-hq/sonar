use crate::ws::{event::LpEvent, IoProxy};
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_raydium_clmm_decoder::accounts::RaydiumClmmAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct RaydiumClmmAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> RaydiumClmmAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for RaydiumClmmAccountProcessor<A> {
    type InputType = AccountProcessorInputType<RaydiumClmmAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let RaydiumClmmAccount::PoolState(pool_state) = account.data {
            let data = LpEvent::from_raydium_clmm(&meta, &pool_state);
            let io = self.io.clone();
            tokio::spawn(async move {
                if let Err(e) = io.broadcast_lp(&data).await {
                    tracing::warn!("Failed to broadcast lp: {}", e);
                }
            });
            return Ok(());
        }
        Ok(())
    }
}
