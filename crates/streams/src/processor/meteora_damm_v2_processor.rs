use crate::ws::{event::LpEvent, IoProxy};
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_meteora_damm_v2_decoder::accounts::MeteoraDammV2Account;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct MeteoraDammV2AccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> MeteoraDammV2AccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for MeteoraDammV2AccountProcessor<A> {
    type InputType = AccountProcessorInputType<MeteoraDammV2Account>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let MeteoraDammV2Account::Pool(pool) = account.data {
            let data = LpEvent::from_meteora_damm_v2(&meta, &pool);
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
