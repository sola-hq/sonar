use crate::constants::{
    METEORA_DAMM_V2_PROGRAM_ID, METEORA_DLMM_PROGRAM_ID, METEORA_POOLS_PROGRAM_ID,
    PUMP_SWAP_PROGRAM_ID, RAYDIUM_AMM_V4_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID,
    RAYDIUM_CPMM_PROGRAM_ID, SYSTEM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID,
};
use carbon_yellowstone_grpc_datasource::{BlockFilters, YellowstoneGrpcGeyserClient};
use std::{
    collections::{HashMap, HashSet},
    env::var,
    sync::Arc,
};
use tokio::sync::RwLock;
use yellowstone_grpc_proto::geyser::{CommitmentLevel, SubscribeRequestFilterAccounts};

pub fn make_geyser_datasource() -> YellowstoneGrpcGeyserClient {
    let endpoint = var("GEYSER_URL").expect("GEYSER_URL is not set");
    let x_token = var("GEYSER_X_TOKEN").ok();

    // Set up transaction filters to swap transactions
    let mut account_filters = HashMap::new();
    account_filters.insert(
        "token_account_filter".to_string(),
        SubscribeRequestFilterAccounts {
            account: vec![],
            owner: vec![
                METEORA_DAMM_V2_PROGRAM_ID.to_string(),
                METEORA_DLMM_PROGRAM_ID.to_string(),
                METEORA_POOLS_PROGRAM_ID.to_string(),
                PUMP_SWAP_PROGRAM_ID.to_string(),
                RAYDIUM_AMM_V4_PROGRAM_ID.to_string(),
                RAYDIUM_CLMM_PROGRAM_ID.to_string(),
                RAYDIUM_CPMM_PROGRAM_ID.to_string(),
                SYSTEM_PROGRAM_ID.to_string(),
                TOKEN_PROGRAM_ID.to_string(),
                TOKEN_2022_PROGRAM_ID.to_string(),
            ],
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );

    let transaction_filters = Default::default();
    let block_filters = BlockFilters { filters: HashMap::new(), failed_transactions: Some(false) };
    let account_deletions_tracked = Arc::new(RwLock::new(HashSet::new()));
    YellowstoneGrpcGeyserClient::new(
        endpoint,
        x_token,
        Some(CommitmentLevel::Processed),
        account_filters,
        transaction_filters,
        block_filters,
        account_deletions_tracked,
    )
}
