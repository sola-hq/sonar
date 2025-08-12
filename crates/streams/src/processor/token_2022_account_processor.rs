use crate::ws::{event::TokenHolderEvent, IoProxy};
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_token_2022_decoder::accounts::Token2022Account;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct Token2022AccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> Token2022AccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for Token2022AccountProcessor<A> {
    type InputType = AccountProcessorInputType<Token2022Account>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, account, _solana_account) = data;

        if let Token2022Account::Token(account) = account.data {
            let data = TokenHolderEvent::from_token_2022_account(meta, account);

            let io = self.io.clone();
            let _ = tokio::spawn(async move {
                let _ = io.broadcast_token_holder(&data).await;
            });
        }
        Ok(())
    }
}
