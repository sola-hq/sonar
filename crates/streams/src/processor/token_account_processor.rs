use crate::ws::{event::TokenHolderEvent, IoProxy};
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_token_program_decoder::accounts::TokenProgramAccount;
use socketioxide::adapter::Adapter;
use std::sync::Arc;

pub struct TokenAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> TokenAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for TokenAccountProcessor<A> {
    type InputType = AccountProcessorInputType<TokenProgramAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, decoded, _solana_account) = data;

        if let TokenProgramAccount::Account(account) = decoded.data {
            let data = TokenHolderEvent::from_token_account(meta, account);
            let io = self.io.clone();
            let _ = tokio::spawn(async move {
                let _ = io.broadcast_token_holder(&data).await;
            });
        }
        Ok(())
    }
}
