use crate::ws::IoProxy;
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_raydium_cpmm_decoder::accounts::RaydiumCpmmAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct RaydiumCpmmAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> RaydiumCpmmAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for RaydiumCpmmAccountProcessor<A> {
    type InputType = AccountProcessorInputType<RaydiumCpmmAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let RaydiumCpmmAccount::PoolState(pool_state) = account.data {
            if let Ok(value) = serde_json::to_value(pool_state) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast Raydium CPMM pool state update: {}", e);
                    }
                });
            }
            return Ok(());
        }
        Ok(())
    }
}
