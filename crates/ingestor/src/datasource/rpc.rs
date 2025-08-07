use solana_client::nonblocking::rpc_client::RpcClient;
use std::env::var;

/// Make a RPC client
///
/// # Arguments
///
/// * `rpc_url` - The URL of the RPC node
pub fn make_rpc_client() -> RpcClient {
    let rpc_url = var("RPC_URL").expect("RPC_URL is not set");
    RpcClient::new(rpc_url)
}
