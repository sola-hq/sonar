use crate::constants::{SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use carbon_rpc_program_subscribe_datasource::{Filters, RpcProgramSubscribe};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_commitment_config::CommitmentConfig;
use std::env::var;

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_token_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(TOKEN_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_token_2022_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(TOKEN_2022_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_system_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(SYSTEM_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}
