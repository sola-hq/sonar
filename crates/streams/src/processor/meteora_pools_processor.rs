use crate::ws::{event::LpEvent, IoProxy};
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_meteora_pools_decoder::accounts::MeteoraPoolsProgramAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct MeteoraPoolsAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> MeteoraPoolsAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for MeteoraPoolsAccountProcessor<A> {
    type InputType = AccountProcessorInputType<MeteoraPoolsProgramAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let MeteoraPoolsProgramAccount::Pool(pool) = account.data {
            let data = LpEvent::from_meteora_pool(&meta, &pool);
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
