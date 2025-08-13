use crate::{
    processor::{
        MeteoraDammV2AccountProcessor, MeteoraDlmmAccountProcessor, MeteoraPoolsAccountProcessor,
        PumpSwapAccountProcessor, RaydiumAmmV4AccountProcessor, RaydiumClmmAccountProcessor,
        RaydiumCpmmAccountProcessor, SystemAccountProcessor, Token2022AccountProcessor,
        TokenAccountProcessor,
    },
    ws::IoProxy,
};
use anyhow::{Context, Result};
use carbon_core::{
    datasource::Datasource,
    pipeline::{Pipeline, ShutdownStrategy},
};
use carbon_log_metrics::LogMetrics;
use carbon_meteora_damm_v2_decoder::MeteoraDammV2Decoder;
use carbon_meteora_dlmm_decoder::MeteoraDlmmDecoder;
use carbon_meteora_pools_decoder::MeteoraPoolsDecoder;
use carbon_pump_swap_decoder::PumpSwapDecoder;
use carbon_raydium_amm_v4_decoder::RaydiumAmmV4Decoder;
use carbon_raydium_clmm_decoder::RaydiumClmmDecoder;
use carbon_raydium_cpmm_decoder::RaydiumCpmmDecoder;
use carbon_system_program_decoder::SystemProgramDecoder;
use carbon_token_2022_decoder::Token2022Decoder;
use carbon_token_program_decoder::TokenProgramDecoder;
use socketioxide::adapter::Adapter;
use std::sync::Arc;
use tracing::info;

pub mod geyser;
pub mod ws;

pub use geyser::make_geyser_datasource;
pub use ws::make_ws_datasource;

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
    let raydium_amm_v4_account_processor = RaydiumAmmV4AccountProcessor::new(io_proxy.clone());
    let raydium_clmm_account_processor = RaydiumClmmAccountProcessor::new(io_proxy.clone());
    let raydium_cpmm_account_processor = RaydiumCpmmAccountProcessor::new(io_proxy.clone());
    let meteora_dlmm_account_processor = MeteoraDlmmAccountProcessor::new(io_proxy.clone());
    let meteora_pools_account_processor = MeteoraPoolsAccountProcessor::new(io_proxy.clone());
    let meteora_damm_v2_account_processor = MeteoraDammV2AccountProcessor::new(io_proxy.clone());
    let pump_swap_account_processor = PumpSwapAccountProcessor::new(io_proxy.clone());

    let pipeline: Pipeline = builder
        .metrics(Arc::new(LogMetrics::new()))
        .shutdown_strategy(ShutdownStrategy::Immediate)
        .channel_buffer_size(channel_buffer_size)
        .account(TokenProgramDecoder, token_account_processor)
        .account(Token2022Decoder, token_2022_account_processor)
        .account(SystemProgramDecoder, system_account_processor)
        .account(RaydiumAmmV4Decoder, raydium_amm_v4_account_processor)
        .account(RaydiumClmmDecoder, raydium_clmm_account_processor)
        .account(RaydiumCpmmDecoder, raydium_cpmm_account_processor)
        .account(MeteoraDlmmDecoder, meteora_dlmm_account_processor)
        .account(MeteoraPoolsDecoder, meteora_pools_account_processor)
        .account(MeteoraDammV2Decoder, meteora_damm_v2_account_processor)
        .account(PumpSwapDecoder, pump_swap_account_processor)
        .build()
        .context("Failed to build pipeline")?;
    Ok(pipeline)
}
