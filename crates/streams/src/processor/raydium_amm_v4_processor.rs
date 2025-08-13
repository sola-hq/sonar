use crate::ws::IoProxy;
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_raydium_amm_v4_decoder::accounts::RaydiumAmmV4Account;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct RaydiumAmmV4AccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> RaydiumAmmV4AccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for RaydiumAmmV4AccountProcessor<A> {
    type InputType = AccountProcessorInputType<RaydiumAmmV4Account>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let RaydiumAmmV4Account::AmmInfo(amm_info) = account.data {
            if let Ok(value) = serde_json::to_value(amm_info) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast Raydium AMM v4 amm info update: {}", e);
                    }
                });
            }
        }
        Ok(())
    }
}
