use crate::ws::IoProxy;
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
            if let Ok(value) = serde_json::to_value(pool) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast Meteora DLMM lb pair update: {}", e);
                    }
                });
            }
            return Ok(());
        }
        Ok(())
    }
}
