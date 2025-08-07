use crate::constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR};
use carbon_helius_atlas_ws_datasource::{Filters, HeliusWebsocket};
use helius::types::{
    Cluster, RpcTransactionsConfig, TransactionCommitment, TransactionDetails,
    TransactionSubscribeFilter, TransactionSubscribeOptions, UiEnhancedTransactionEncoding,
};
use std::{collections::HashSet, env::var, sync::Arc};
use tokio::sync::RwLock;

const HELIUS_PING_INTERVAL_SECS: u64 = 10; // 10 seconds
const HELIUS_PONG_TIMEOUT_SECS: u64 = 30; // 30 seconds
const HELIUS_TRANSACTION_IDLE_TIMEOUT_SECS: u64 = 10; // 10 seconds

/// Make a helius websocket datasource
///
/// # Arguments
///
/// * `helius_atlas_ws_url` - The URL of the Helius Atlas websocket
/// * `helius_atlas_api_key` - The API key for the Helius Atlas websocket
pub fn make_helius_ws_datasource() -> HeliusWebsocket {
    let transaction_filters = Some(RpcTransactionsConfig {
        filter: TransactionSubscribeFilter {
            account_include: Some(vec![
                USDC_MINT_KEY_STR.to_string(),
                USDT_MINT_KEY_STR.to_string(),
                WSOL_MINT_KEY_STR.to_string(),
            ]),
            account_exclude: None,
            account_required: None,
            vote: None,
            failed: None,
            signature: None,
        },
        options: TransactionSubscribeOptions {
            commitment: Some(TransactionCommitment::Confirmed),
            encoding: Some(UiEnhancedTransactionEncoding::Base64),
            transaction_details: Some(TransactionDetails::Full),
            show_rewards: None,
            max_supported_transaction_version: Some(0),
        },
    });
    let filters = Filters::new(vec![], transaction_filters)
        .expect("Error creating Filters for the Helius WebSocket");
    let api_key = var("HELIUS_ATLAS_API_KEY").expect("HELIUS_ATLAS_API_KEY is not set");
    let ping_interval_secs = var("HELIUS_PING_INTERVAL_SECS")
        .map(|v| v.parse::<u64>().unwrap_or(HELIUS_PING_INTERVAL_SECS))
        .unwrap_or(HELIUS_PING_INTERVAL_SECS);
    let pong_timeout_secs = var("HELIUS_PONG_TIMEOUT_SECS")
        .map(|v| v.parse::<u64>().unwrap_or(HELIUS_PONG_TIMEOUT_SECS))
        .unwrap_or(HELIUS_PONG_TIMEOUT_SECS);
    let transaction_idle_timeout_secs = var("HELIUS_TRANSACTION_IDLE_TIMEOUT_SECS")
        .map(|v| v.parse::<u64>().unwrap_or(HELIUS_TRANSACTION_IDLE_TIMEOUT_SECS))
        .unwrap_or(HELIUS_TRANSACTION_IDLE_TIMEOUT_SECS);

    HeliusWebsocket::new(
        api_key,
        Some(ping_interval_secs),
        Some(pong_timeout_secs),
        Some(transaction_idle_timeout_secs),
        filters,
        Arc::new(RwLock::new(HashSet::new())),
        Cluster::MainnetBeta,
    )
    .with_ping_interval_secs(ping_interval_secs)
    .with_pong_timeout_secs(pong_timeout_secs)
    .with_transaction_idle_timeout_secs(transaction_idle_timeout_secs)
}
