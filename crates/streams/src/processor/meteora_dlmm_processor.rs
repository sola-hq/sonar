use crate::ws::IoProxy;
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_meteora_dlmm_decoder::accounts::MeteoraDlmmAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct MeteoraDlmmAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> MeteoraDlmmAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for MeteoraDlmmAccountProcessor<A> {
    type InputType = AccountProcessorInputType<MeteoraDlmmAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let MeteoraDlmmAccount::LbPair(lb_pair) = account.data {
            if let Ok(value) = serde_json::to_value(&lb_pair) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast Meteora DAMM v2 pool update: {}", e);
                    }
                });
            }
            return Ok(());
        }
        Ok(())
    }
}
