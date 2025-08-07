use crate::constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR};
use carbon_yellowstone_grpc_datasource::{BlockFilters, YellowstoneGrpcGeyserClient};
use std::{
    collections::{HashMap, HashSet},
    env::var,
    sync::Arc,
};
use tokio::sync::RwLock;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions,
};

pub fn make_geyser_datasource() -> YellowstoneGrpcGeyserClient {
    let endpoint = var("GEYSER_URL").expect("GEYSER_URL is not set");
    let x_token = var("GEYSER_X_TOKEN").ok();

    // Set up transaction filters to swap transactions
    let mut transaction_filters = HashMap::new();
    transaction_filters.insert(
        "swap_transaction_filter".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![
                USDC_MINT_KEY_STR.to_string(),
                USDT_MINT_KEY_STR.to_string(),
                WSOL_MINT_KEY_STR.to_string(),
            ],
            account_exclude: vec![],
            account_required: vec![],
            signature: None,
        },
    );
    // Create empty account filters since we only care about transactions
    let account_filters: HashMap<String, SubscribeRequestFilterAccounts> = HashMap::new();

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
