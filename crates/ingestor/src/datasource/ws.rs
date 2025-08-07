use carbon_rpc_block_subscribe_datasource::{Filters, RpcBlockSubscribe};
use solana_client::rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter};
use std::env::var;

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_ws_datasource() -> RpcBlockSubscribe {
    let filters = Filters::new(
        RpcBlockSubscribeFilter::All,
        Some(RpcBlockSubscribeConfig {
            max_supported_transaction_version: Some(0),
            ..RpcBlockSubscribeConfig::default()
        }),
    );
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcBlockSubscribe::new(rpc_ws_url, filters)
}
