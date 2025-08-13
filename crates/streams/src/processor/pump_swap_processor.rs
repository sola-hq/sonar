use crate::ws::IoProxy;
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_pump_swap_decoder::accounts::PumpSwapAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct PumpSwapAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> PumpSwapAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for PumpSwapAccountProcessor<A> {
    type InputType = AccountProcessorInputType<PumpSwapAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let PumpSwapAccount::Pool(pool) = account.data {
            if let Ok(value) = serde_json::to_value(pool) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast Pump Swap pool update: {}", e);
                    }
                });
            }
            return Ok(());
        }
        Ok(())
    }
}
