use crate::ws::{event::TokenHolderEvent, IoProxy};
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_system_program_decoder::accounts::SystemAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct SystemAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> SystemAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for SystemAccountProcessor<A> {
    type InputType = AccountProcessorInputType<SystemAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, solana_account) = data;

        if let SystemAccount::Legacy(_) = account.data {
            if let Ok(value) = serde_json::to_value(solana_account) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast token holder: {}", e);
                    }
                });
            }
        }

        Ok(())
    }
}
