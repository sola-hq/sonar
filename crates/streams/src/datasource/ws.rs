use crate::constants::{
    METEORA_DAMM_V2_PROGRAM_ID, METEORA_DLMM_PROGRAM_ID, PUMP_SWAP_PROGRAM_ID,
    RAYDIUM_AMM_V4_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID,
};
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

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_raydium_amm_v4_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(RAYDIUM_AMM_V4_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_raydium_clmm_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(RAYDIUM_CLMM_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_raydium_cpmm_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(RAYDIUM_CPMM_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_meteora_dlmm_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(METEORA_DLMM_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_meteora_pools_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(METEORA_DLMM_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_meteora_damm_v2_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(METEORA_DAMM_V2_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_pump_swap_account_subscribe_datasource() -> RpcProgramSubscribe {
    let account_subscribe_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };
    let filters = Filters::new(PUMP_SWAP_PROGRAM_ID, Some(account_subscribe_config));
    let rpc_ws_url = var("RPC_WS_URL").expect("RPC_WS_URL is not set");
    RpcProgramSubscribe::new(rpc_ws_url, filters)
}

/// Make a websocket datasource
///
/// # Arguments
///
/// * `rpc_ws_url` - The URL of the RPC websocket
pub fn make_ws_datasource() -> Vec<RpcProgramSubscribe> {
    vec![
        // make_token_account_subscribe_datasource(),
        // make_token_2022_account_subscribe_datasource(),
        // make_system_account_subscribe_datasource(),
        make_raydium_amm_v4_account_subscribe_datasource(),
        // make_raydium_clmm_account_subscribe_datasource(),
        // make_raydium_cpmm_account_subscribe_datasource(),
        // make_meteora_dlmm_account_subscribe_datasource(),
        // make_meteora_pools_account_subscribe_datasource(),
        // make_meteora_damm_v2_account_subscribe_datasource(),
        // make_pump_swap_account_subscribe_datasource(),
    ]
}
