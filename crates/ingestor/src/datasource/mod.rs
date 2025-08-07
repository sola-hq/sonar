use crate::{
    metrics::NodeMetrics,
    processor::{
        MeteoraDlmmInstructionProcessor, MeteoraPoolsInstructionProcessor,
        OcraWhirlpoolInstructionProcessor, PumpAmmInstructionProcessor,
        RaydiumAmmV4InstructionProcessor, RaydiumClmmInstructionProcessor,
        RaydiumCpmmInstructionProcessor, RaydiumLaunchpadInstructionProcessor,
    },
    TokenSwapHandler,
};
use anyhow::Result;
use carbon_core::{
    datasource::Datasource,
    pipeline::{Pipeline, ShutdownStrategy},
};
use carbon_log_metrics::LogMetrics;
use carbon_meteora_dlmm_decoder::MeteoraDlmmDecoder;
use carbon_meteora_pools_decoder::MeteoraPoolsDecoder;
use carbon_orca_whirlpool_decoder::OrcaWhirlpoolDecoder;
use carbon_pump_swap_decoder::PumpSwapDecoder;
use carbon_raydium_amm_v4_decoder::RaydiumAmmV4Decoder;
use carbon_raydium_clmm_decoder::RaydiumClmmDecoder;
use carbon_raydium_cpmm_decoder::RaydiumCpmmDecoder;
use carbon_raydium_launchpad_decoder::RaydiumLaunchpadDecoder;
use sonar_db::{Database, KvStore, MessageQueue};
use std::sync::Arc;

pub mod block;
pub mod geyser;
pub mod helius;
pub mod rpc;
pub mod tx;
pub mod ws;

pub fn build_pipeline<DS>(
    datasource: DS,
    db: Arc<Database>,
    kv_store: Arc<KvStore>,
    message_queue: Arc<MessageQueue>,
) -> Result<Pipeline>
where
    DS: Datasource + Send + Sync + 'static,
{
    let channel_buffer_size = std::env::var("PIPELINE_CHANNEL_BUFFER_SIZE")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<usize>()
        .unwrap_or(10_000);
    let metrics = Arc::new(NodeMetrics::new());
    let token_swap_handler = Arc::new(TokenSwapHandler::new(
        kv_store.clone(),
        message_queue.clone(),
        db.clone(),
        metrics,
    ));
    let pipeline: Pipeline = Pipeline::builder()
        .datasource(datasource)
        .metrics(Arc::new(LogMetrics::new()))
        .shutdown_strategy(ShutdownStrategy::Immediate)
        .channel_buffer_size(channel_buffer_size)
        .instruction(
            RaydiumAmmV4Decoder,
            RaydiumAmmV4InstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(
            RaydiumClmmDecoder,
            RaydiumClmmInstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(
            RaydiumCpmmDecoder,
            RaydiumCpmmInstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(
            RaydiumLaunchpadDecoder,
            RaydiumLaunchpadInstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(
            MeteoraDlmmDecoder,
            MeteoraDlmmInstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(
            MeteoraPoolsDecoder,
            MeteoraPoolsInstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(
            OrcaWhirlpoolDecoder,
            OcraWhirlpoolInstructionProcessor::new(token_swap_handler.clone()),
        )
        .instruction(PumpSwapDecoder, PumpAmmInstructionProcessor::new(token_swap_handler.clone()))
        .build()?;
    Ok(pipeline)
}
