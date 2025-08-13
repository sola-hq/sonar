use crate::{
    processor::{SystemAccountProcessor, Token2022AccountProcessor, TokenAccountProcessor},
    ws::IoProxy,
};
use anyhow::{Context, Result};
use carbon_core::{
    datasource::Datasource,
    pipeline::{Pipeline, ShutdownStrategy},
};
use carbon_log_metrics::LogMetrics;
use carbon_system_program_decoder::SystemProgramDecoder;
use carbon_token_2022_decoder::Token2022Decoder;
use carbon_token_program_decoder::TokenProgramDecoder;
use socketioxide::adapter::Adapter;
use std::sync::Arc;
use tracing::info;

pub mod account;

pub fn build_pipeline<DS, A: Adapter>(
    datasources: Vec<DS>,
    io_proxy: Arc<IoProxy<A>>,
) -> Result<Pipeline>
where
    DS: Datasource + Send + Sync + 'static,
{
    let channel_buffer_size = std::env::var("PIPELINE_CHANNEL_BUFFER_SIZE")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<usize>()
        .unwrap_or(10_000);

    info!("Building pipeline with channel buffer size: {}", channel_buffer_size);

    let mut builder = Pipeline::builder();
    for ds in datasources.into_iter() {
        builder = builder.datasource(ds);
    }

    info!("Configuring pipeline with Token2022, Token, and System program decoders");

    let token_account_processor = TokenAccountProcessor::new(io_proxy.clone());
    let token_2022_account_processor = Token2022AccountProcessor::new(io_proxy.clone());
    let system_account_processor = SystemAccountProcessor::new(io_proxy.clone());

    let pipeline: Pipeline = builder
        .metrics(Arc::new(LogMetrics::new()))
        .shutdown_strategy(ShutdownStrategy::Immediate)
        .channel_buffer_size(channel_buffer_size)
        .account(TokenProgramDecoder, token_account_processor)
        .account(Token2022Decoder, token_2022_account_processor)
        .account(SystemProgramDecoder, system_account_processor)
        .build()
        .context("Failed to build pipeline")?;
    Ok(pipeline)
}
