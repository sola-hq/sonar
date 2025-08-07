use carbon_rpc_block_crawler_datasource::{RpcBlockConfig, RpcBlockCrawler};
use solana_commitment_config::CommitmentConfig;
use solana_transaction_status::UiTransactionEncoding;
use std::{env::var, time::Duration};

/// Make a block crawler datasource
///
/// # Arguments
///
/// * `rpc_url` - The URL of the RPC node
/// * `start_slot` - The start slot of the block crawler
/// * `end_slot` - The end slot of the block crawler
/// * `block_interval` - The interval of the block crawler
/// * `max_concurrent_requests` - The maximum number of concurrent requests of the block crawler
pub fn make_block_crawler_datasource() -> RpcBlockCrawler {
    let rpc_url = var("RPC_URL").expect("RPC_URL is not set");
    let start_slot = var("RPC_START_SLOT")
        .expect("RPC_START_SLOT is not set")
        .parse::<u64>()
        .expect("RPC_START_SLOT is not a valid number");
    let end_slot = var("RPC_END_SLOT")
        .ok()
        .map(|s| s.parse::<u64>().expect("RPC_END_SLOT is not a valid number"));
    let max_concurrent_requests = var("RPC_MAX_CONCURRENT_REQUESTS")
        .expect("RPC_MAX_CONCURRENT_REQUESTS is not set")
        .parse::<usize>()
        .expect("RPC_MAX_CONCURRENT_REQUESTS is not a valid number");
    let block_interval = var("RPC_BLOCK_INTERVAL")
        .expect("RPC_BLOCK_INTERVAL is not set")
        .parse::<u64>()
        .expect("RPC_BLOCK_INTERVAL is not a valid number");
    let channel_buffer_size = var("RPC_CHANNEL_BUFFER_SIZE")
        .ok()
        .map(|s| s.parse::<usize>().expect("RPC_CHANNEL_BUFFER_SIZE is not a valid number"));

    let block_config = RpcBlockConfig {
        rewards: Some(false),
        encoding: Some(UiTransactionEncoding::Binary),
        max_supported_transaction_version: Some(0),
        commitment: Some(CommitmentConfig::processed()),
        ..Default::default()
    };

    RpcBlockCrawler::new(
        rpc_url,
        start_slot,
        end_slot,
        Some(Duration::from_secs(block_interval)),
        block_config,
        Some(max_concurrent_requests),
        channel_buffer_size,
    )
}
