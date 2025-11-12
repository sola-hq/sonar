use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, USDT_SET, WSOL_MINT_KEY_STR},
    decoder::{
        extra_mint_details_from_tx_metadata, MintDetail, TokenTransferDetails, SPL_TOKEN_DECODER,
    },
    metrics::NodeMetrics,
};
use anyhow::Result;
// use backon::{ExponentialBuilder, Retryable};
use carbon_core::{
    instruction::{InstructionMetadata, NestedInstruction},
    transaction::TransactionMetadata,
};
use chrono::Utc;
use sonar_db::{models::NewPoolEvent, Database, KvStore, MessageQueue, SwapEvent, Trade};
use sonar_sol_price::get_sol_price;
use sonar_token_metadata::get_token_metadata_with_data;
use std::collections::HashMap;
use std::{collections::HashSet, sync::Arc};
use tracing::{debug, error};

const TINY_SWAP_UI_AMOUNT: f64 = 0.01; // 0.01 SOL
const TINY_SWAP_AMOUNT: f64 = 0.1; // 0.1 USDC

#[derive(Clone)]
pub struct TokenSwapAccounts {
    pub pair: String,
    pub user_adas: HashSet<String>,
    pub vault_adas: HashSet<String>,
    pub fee_adas: Option<HashSet<String>>,
    pub quote_mints: Arc<HashSet<String>>,
}

#[derive(Clone)]
pub struct TokenSwapHandler {
    pub kv_store: Arc<KvStore>,
    pub message_queue: Arc<MessageQueue>,
    pub db: Arc<Database>,
    pub metrics: Arc<NodeMetrics>,
}

impl TokenSwapHandler {
    pub fn new(
        kv_store: Arc<KvStore>,
        message_queue: Arc<MessageQueue>,
        db: Arc<Database>,
        metrics: Arc<NodeMetrics>,
    ) -> Self {
        Self { kv_store, message_queue, db, metrics }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_swap_instruction(
        &self,
        token_swap_accounts: &TokenSwapAccounts,
        meta: &InstructionMetadata,
        nested_instructions: &[NestedInstruction],
    ) {
        debug!("https://solscan.io/tx/{}", meta.transaction_metadata.signature);

        let message_queue = self.message_queue.clone();
        let kv_store = self.kv_store.clone();
        let db = self.db.clone();
        let metrics = self.metrics.clone();
        let token_swap_accounts = token_swap_accounts.clone();
        let transaction_metadata = meta.transaction_metadata.clone();
        let nested_instructions = nested_instructions.to_vec();

        metrics.increment_total_swaps();

        tokio::spawn(async move {
            match process_token_swap_instruction(
                &token_swap_accounts,
                &transaction_metadata,
                &nested_instructions,
                &message_queue,
                &kv_store,
                &db,
                &metrics,
            )
            .await
            {
                Ok(_) => {
                    metrics.increment_succeed_swaps();
                }
                Err(e) => {
                    metrics.increment_failed_swaps();
                    error!(
                        ?e,
                        "Transaction: https://solscan.io/tx/{}", transaction_metadata.signature
                    );
                }
            }
        });
    }

    pub fn spawn_new_pool_instruction(&self, _meta: &InstructionMetadata, event: NewPoolEvent) {
        let message_queue = self.message_queue.clone();
        tokio::spawn(async move {
            if let Err(e) = message_queue.publish_new_pool(&event).await {
                error!("Failed to publish new pool event: {:?}", e);
            }
        });
    }
}

pub struct SwapResult {
    pub price: f64,
    pub base: String,
    pub base_amount: f64,
    pub quote: String,
    pub quote_amount: f64,
    pub swap_amount: f64,
    pub is_buy: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("Expected 2 token swaps")]
    ExpectedTwoTokenSwaps,
    #[error("Tiny swap")]
    TinySwap,
    #[error("Zero swap")]
    ZeroSwap,
    #[error("Unexpected swap")]
    UnexpectedSwap,
    #[error("Db insert failure")]
    DbInsertFailure(anyhow::Error),
    #[error("Message send failure")]
    MessageSendFailure(anyhow::Error),
    #[error("Kv insert failure")]
    KvInsertFailure(anyhow::Error),
    #[error("Token metadata failure")]
    TokenMetadataFailure(anyhow::Error),
}

/// Updates the metrics for a swap error.
///
/// # Arguments
///
/// * `metrics` - The metrics to update
/// * `e` - The swap error
fn update_metrics_for_swap_error(metrics: &NodeMetrics, e: SwapError) {
    match e {
        SwapError::TinySwap => metrics.increment_skipped_tiny_swaps(),
        SwapError::ZeroSwap => metrics.increment_skipped_zero_swaps(),
        SwapError::TokenMetadataFailure(_) => metrics.increment_skipped_no_metadata(),
        SwapError::UnexpectedSwap => metrics.increment_skipped_unexpected_swaps(),
        SwapError::ExpectedTwoTokenSwaps => metrics.increment_skipped_unknown_swaps(),
        SwapError::DbInsertFailure(_) => metrics.increment_db_insert_failure(),
        SwapError::MessageSendFailure(_) => metrics.increment_message_send_failure(),
        SwapError::KvInsertFailure(_) => metrics.increment_kv_insert_failure(),
    }
}

/// Extracts all token transfers from a transaction's nested instructions.
///
/// This function processes both the outer instructions and all nested inner instructions
/// recursively to collect all token transfers that occurred in the transaction.
///
/// # Arguments
///
/// * `transaction_metadata` - The metadata of the transaction containing account information
/// * `nested_instructions` - The list of instructions to process, including their nested instructions
///
/// # Returns
///
/// A vector of `TokenTransferDetails` containing all token transfers found in the transaction
pub fn get_inner_token_transfers(
    transaction_metadata: &TransactionMetadata,
    nested_instructions: &[NestedInstruction],
) -> Vec<TokenTransferDetails> {
    let mint_details = extra_mint_details_from_tx_metadata(transaction_metadata);
    recursive_inner_token_transfers(nested_instructions, &mint_details)
}

/// Recursively processes nested instructions to extract token transfers.
///
/// This internal function handles the recursive traversal of nested instructions
/// to collect all token transfers. It processes both the current level instructions
/// and recursively processes all nested instructions.
///
/// # Arguments
///
/// * `nested_instructions` - The list of instructions to process
/// * `mint_details` - A map containing mint account details for token identification
///
/// # Returns
///
/// A vector of `TokenTransferDetails` containing all token transfers found in the current
/// level and all nested levels of instructions
fn recursive_inner_token_transfers(
    nested_instructions: &[NestedInstruction],
    mint_details: &HashMap<String, MintDetail>,
) -> Vec<TokenTransferDetails> {
    let mut transfers: Vec<TokenTransferDetails> = Vec::new();

    // Process current level instructions
    let current_transfers = SPL_TOKEN_DECODER
        .decode_token_transfers_from_instructions(nested_instructions, mint_details);
    transfers.extend(current_transfers);

    // Recursively process nested instructions
    for nested_instruction in nested_instructions {
        transfers.extend(recursive_inner_token_transfers(
            &nested_instruction.inner_instructions,
            mint_details,
        ));
    }
    transfers
}

/// Checks if the swap is valid.
///
/// # Arguments
///
/// * `transfers` - The list of token transfers
/// * `transaction_metadata` - The transaction metadata
///
/// # Returns
///
/// A boolean indicating if the swap is valid.
pub fn is_valid_swap(
    transfers: &[TokenTransferDetails],
    transaction_metadata: &TransactionMetadata,
) -> Result<(), SwapError> {
    if transfers.len() != 2 {
        debug!(
            "https://solscan.io/tx/{} skipping swap with unexpected number of tokens: {}",
            transaction_metadata.signature,
            transfers.len()
        );
        return Err(SwapError::ExpectedTwoTokenSwaps);
    }

    if transfers.iter().all(|d| d.ui_amount < TINY_SWAP_UI_AMOUNT) {
        debug!("skipping tiny swaps");
        return Err(SwapError::TinySwap);
    }

    if transfers.iter().any(|d| d.ui_amount == 0.0) {
        debug!("skipping zero swaps (arbitrage likely)");
        return Err(SwapError::ZeroSwap);
    }

    Ok(())
}

// https://solscan.io/tx/2usSAGxq35GJxQxVKHQ7NHBDnJim95Jyk3AeFrRAcpHc2TJUH3bjhVSvtAWcxnqnQyJFzpPFgJvMHNkTuQ8t779f
pub fn is_swap_inner_transfer(
    transfer: &TokenTransferDetails,
    user_adas: &HashSet<String>,
    vaults_adas: &HashSet<String>,
    fee_adas: Option<&HashSet<String>>,
) -> bool {
    // Early return if it's a fee transfer
    if let Some(fee_adas) = fee_adas {
        if fee_adas.contains(&transfer.destination) {
            return false;
        }
    }
    // Check if it's a user transfer
    (user_adas.contains(&transfer.destination) || user_adas.contains(&transfer.source))
        // Check if it's a vault transfer
        && (vaults_adas.contains(&transfer.destination) || vaults_adas.contains(&transfer.source))
}

pub fn build_swap_event(
    pair: &str,
    is_buy: bool,
    base: &TokenTransferDetails,
    quote: &TokenTransferDetails,
    quote_price: f64,
    transaction_metadata: &TransactionMetadata,
) -> SwapEvent {
    let is_pump = base.mint.to_lowercase().ends_with("pump");
    let base_amount = base.ui_amount;
    let quote_amount = quote.ui_amount;

    let price = (quote_amount / base_amount) * quote_price;
    let swap_amount = quote_amount * quote_price;

    let signers = transaction_metadata
        .message
        .static_account_keys()
        .iter()
        .take(transaction_metadata.message.header().num_required_signatures as usize)
        .map(|pubkey| pubkey.to_string())
        .collect::<Vec<String>>();

    SwapEvent {
        pair: pair.to_string(),
        pubkey: base.mint.clone(),
        price,
        market_cap: 0.0,
        timestamp: transaction_metadata.block_time.unwrap_or(Utc::now().timestamp()) as u64,
        slot: transaction_metadata.slot,
        base_amount,
        quote_amount,
        swap_amount,
        owner: transaction_metadata.fee_payer.to_string(),
        signature: transaction_metadata.signature.to_string(),
        signers,
        is_pump,
        is_buy,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn get_swap_event_with_token_transfer_details(
    token_swap_accounts: &TokenSwapAccounts,
    transfers: &[TokenTransferDetails],
    transaction_metadata: &TransactionMetadata,
    kv_store: &Arc<KvStore>,
    db: &Arc<Database>,
) -> Result<SwapEvent, SwapError> {
    is_valid_swap(transfers, transaction_metadata)?;

    let (is_buy, base_mint_details, quote_mint_details) =
        get_base_quote_mint(token_swap_accounts, transfers)?;
    let (_quote_mint, quote_price) = get_quote_price(
        quote_mint_details.mint.as_str(),
        Some(transaction_metadata.block_time.unwrap_or(Utc::now().timestamp()) as u64),
        kv_store,
    )
    .await;

    let mut swap_event = build_swap_event(
        &token_swap_accounts.pair,
        is_buy,
        base_mint_details,
        quote_mint_details,
        quote_price,
        transaction_metadata,
    );

    // let f = || get_token_metadata_with_data(swap_event.pubkey.as_str(), kv_store, db);
    // let supply = match f.retry(ExponentialBuilder::default()).await {
    //     Ok(token) => token.supply,
    //     Err(e) => {
    //         error!("Failed to get token metadata for {} {:?}", swap_event.pubkey, e);
    //         0.0
    //     }
    // };

    let supply = match get_token_metadata_with_data(swap_event.pubkey.as_str(), kv_store, db).await
    {
        Ok(token) => token.supply,
        Err(e) => {
            error!("Failed to get token metadata for {} {:?}", swap_event.pubkey, e);
            0.0
        }
    };

    swap_event.update_market_cap(supply);

    // Skip tiny swaps
    if swap_event.swap_amount < TINY_SWAP_AMOUNT {
        return Err(SwapError::TinySwap);
    }

    Ok(swap_event)
}

/// Filters token transfers to only include valid swap transfers.
///
/// # Arguments
///
/// * `transfers` - The list of token transfers to filter
/// * `token_swap_accounts` - The token swap accounts to use for filtering
///
/// # Returns
///
/// A new vector containing only the valid swap transfers.
pub fn filter_swap_transfers(
    transfers: &[TokenTransferDetails],
    token_swap_accounts: &TokenSwapAccounts,
) -> Vec<TokenTransferDetails> {
    transfers
        .iter()
        .filter(|t| {
            is_swap_inner_transfer(
                t,
                &token_swap_accounts.user_adas,
                &token_swap_accounts.vault_adas,
                token_swap_accounts.fee_adas.as_ref(),
            )
        })
        .cloned()
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub async fn process_token_swap_instruction(
    token_swap_accounts: &TokenSwapAccounts,
    transaction_metadata: &TransactionMetadata,
    nested_instructions: &[NestedInstruction],
    message_queue: &Arc<MessageQueue>,
    kv_store: &Arc<KvStore>,
    db: &Arc<Database>,
    metrics: &NodeMetrics,
) -> Result<(), SwapError> {
    let transfers = get_inner_token_transfers(transaction_metadata, nested_instructions);
    let filtered_transfers = filter_swap_transfers(&transfers, token_swap_accounts);

    let swap_event = match get_swap_event_with_token_transfer_details(
        token_swap_accounts,
        &filtered_transfers,
        transaction_metadata,
        kv_store,
        db,
    )
    .await
    {
        Ok(swap_event) => swap_event,
        Err(e) => {
            update_metrics_for_swap_error(metrics, e);
            return Ok(());
        }
    };

    match db.insert_swap_event(&swap_event).await {
        Ok(_) => metrics.increment_db_insert_success(),
        Err(e) => {
            metrics.increment_db_insert_failure();
            return Err(SwapError::DbInsertFailure(e));
        }
    };

    let trade: Trade = swap_event.into();
    match message_queue.publish_trade(&trade).await {
        Ok(_) => metrics.increment_message_send_success(),
        Err(e) => {
            metrics.increment_message_send_failure();
            return Err(SwapError::MessageSendFailure(e));
        }
    }

    match kv_store.insert_price(&trade).await {
        Ok(_) => metrics.increment_kv_insert_success(),
        Err(e) => {
            metrics.increment_kv_insert_failure();
            return Err(SwapError::KvInsertFailure(e));
        }
    }
    Ok(())
}

pub fn get_base_quote_mint<'a>(
    token_swap_accounts: &TokenSwapAccounts,
    transfers: &'a [TokenTransferDetails],
) -> Result<(bool, &'a TokenTransferDetails, &'a TokenTransferDetails), SwapError> {
    let (token0, token1) = (&transfers[0], &transfers[1]);
    let (mut base_mint, mut quote_mint) = match (
        token_swap_accounts.quote_mints.contains(&token0.mint),
        token_swap_accounts.quote_mints.contains(&token1.mint),
    ) {
        (_, true) => (token0, token1),
        (true, false) => (token1, token0),
        _ => return Err(SwapError::UnexpectedSwap),
    };

    // this is to handle the case where the quote mint is WSOL and the base mint is USDC or USDT
    if quote_mint.mint == WSOL_MINT_KEY_STR && USDT_SET.contains(&base_mint.mint) {
        (base_mint, quote_mint) = (quote_mint, base_mint);
    }

    let is_buy = token_swap_accounts.vault_adas.contains(&quote_mint.destination)
        || token_swap_accounts.vault_adas.contains(&base_mint.source);
    Ok((is_buy, base_mint, quote_mint))
}

#[cfg(not(feature = "hist"))]
pub async fn get_quote_price(
    quote_mint: &str,
    _timestamp: Option<u64>,
    _kv_store: &Arc<KvStore>,
) -> (String, f64) {
    if quote_mint == WSOL_MINT_KEY_STR {
        let quote_price = get_sol_price().await;
        (WSOL_MINT_KEY_STR.to_string(), quote_price)
    } else if quote_mint == USDC_MINT_KEY_STR {
        (USDC_MINT_KEY_STR.to_string(), 1.0)
    } else if quote_mint == USDT_MINT_KEY_STR {
        (USDT_MINT_KEY_STR.to_string(), 1.0)
    } else {
        // TODO: add support for other mints
        (quote_mint.to_string(), 0.0)
    }
}

#[cfg(feature = "hist")]
pub async fn get_quote_price(
    quote_mint: &str,
    timestamp: Option<u64>,
    kv_store: &Arc<KvStore>,
) -> (String, f64) {
    if quote_mint == USDC_MINT_KEY_STR {
        (USDC_MINT_KEY_STR.to_string(), 1.0)
    } else if quote_mint == USDT_MINT_KEY_STR {
        (USDT_MINT_KEY_STR.to_string(), 1.0)
    } else if quote_mint == WSOL_MINT_KEY_STR {
        if let Some(timestamp) = timestamp {
            match kv_store.get_price_at_timestamp(quote_mint, timestamp).await {
                Ok(price) => {
                    return (quote_mint.to_string(), price);
                }
                Err(e) => {
                    error!("historical error {:?}", e);
                }
            }
        }
        let quote_price = get_sol_price().await;
        (WSOL_MINT_KEY_STR.to_string(), quote_price)
    } else {
        // TODO: add support for other mints
        (quote_mint.to_string(), 0.0)
    }
}

pub async fn save_swap_event(
    kv_store: Arc<KvStore>,
    message_queue: Arc<MessageQueue>,
    db: Arc<Database>,
    metrics: Arc<NodeMetrics>,
    swap_event: SwapEvent,
) {
    let trade: Trade = swap_event.clone().into();
    let swap_event_clone = swap_event.clone();

    match db.insert_swap_event(&swap_event_clone).await {
        Ok(_) => metrics.increment_db_insert_success(),
        Err(e) => {
            metrics.increment_db_insert_failure();
            error!("Failed to insert swap event: {}", e);
        }
    }

    match message_queue.publish_trade(&trade).await {
        Ok(_) => metrics.increment_message_send_success(),
        Err(e) => {
            metrics.increment_message_send_failure();
            error!("Failed to send swap event to message queue: {}", e.to_string());
        }
    }

    match kv_store.insert_price(&trade).await {
        Ok(_) => metrics.increment_kv_insert_success(),
        Err(e) => {
            metrics.increment_kv_insert_failure();
            error!("Failed to insert swap event into kv store: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::ToPrimitive;
    use std::ops::Div;

    #[tokio::test]
    async fn test_sell_swap() {
        let user_adas = HashSet::from([
            "yAcYcbC9Qr9SBpeG9SbT1zAEFwHd8j6EFFWomjQjVtn".to_string(),
            "6qxghyVLU7sVYhQn6JKziDqb2VMPuDS6Q6rGngnkXdxx".to_string(),
        ]);
        let vaults_adas = HashSet::from([
            "GHs3Cs9J6NoX79Nr2KvR1Nnzm82R34Jmqh1A8Bb84zgc".to_string(),
            "4UKfPxrJGEXggv637xCbzethVUGtkv6vay5zCjDSg1Yb".to_string(),
        ]);
        let fee_adas = HashSet::from(["Bvtgim23rfocUzxVX9j9QFxTbBnH8JZxnaGLCEkXvjKS".to_string()]);
        let transfers = vec![
            TokenTransferDetails {
                amount: 2523000000,
                ui_amount: 2523.0,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "G2gUder2Y934cm8ufSQxjbhjrfJsiBBAox1jgLqEDx75".to_string(),
                destination: "GHs3Cs9J6NoX79Nr2KvR1Nnzm82R34Jmqh1A8Bb84zgc".to_string(),
                mint: "2WZuixz3wohXbib7Ze2gRjVeGeESiMw9hsizDwbjM4YK".to_string(),
                source: "yAcYcbC9Qr9SBpeG9SbT1zAEFwHd8j6EFFWomjQjVtn".to_string(),
            },
            TokenTransferDetails {
                amount: 7229486,
                ui_amount: 0.007229486,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "ezWtvReswwZaEBThCnW23qtH5uANic2akGY7yh7vZR9".to_string(),
                destination: "6qxghyVLU7sVYhQn6JKziDqb2VMPuDS6Q6rGngnkXdxx".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "4UKfPxrJGEXggv637xCbzethVUGtkv6vay5zCjDSg1Yb".to_string(),
            },
            TokenTransferDetails {
                amount: 3624,
                ui_amount: 0.000003624,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "ezWtvReswwZaEBThCnW23qtH5uANic2akGY7yh7vZR9".to_string(),
                destination: "Bvtgim23rfocUzxVX9j9QFxTbBnH8JZxnaGLCEkXvjKS".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "4UKfPxrJGEXggv637xCbzethVUGtkv6vay5zCjDSg1Yb".to_string(),
            },
        ];
        let is_valid =
            is_swap_inner_transfer(&transfers[0], &user_adas, &vaults_adas, Some(&fee_adas));
        assert!(is_valid, "token should be valid");

        let is_valid =
            is_swap_inner_transfer(&transfers[1], &user_adas, &vaults_adas, Some(&fee_adas));
        assert!(is_valid, "wsol ix should be valid");

        let is_valid =
            is_swap_inner_transfer(&transfers[2], &user_adas, &vaults_adas, Some(&fee_adas));
        assert!(!is_valid, "the fee ix should be invalid");
    }

    #[tokio::test]
    async fn test_buy_swap() {
        let user_adas = HashSet::from([
            "9qr6mtX3fELoWGQJyVzHgxuQZptZhmHRMdgZNyGDZkjB".to_string(),
            "GHjM41KiTeTiRR2m42RQF4jSpho4C4KKSx4D1ZX7D3Qb".to_string(),
        ]);
        let vaults_adas = HashSet::from([
            "GkcKiF8ku7e54A8NK4UPHW6rmoGfhMeiMHGPpn4yUTkG".to_string(),
            "39NaF7ehkzNcxXLq9WZdtQ18RFu1rVxs3oQR1a2safoT".to_string(),
        ]);
        let fee_adas = HashSet::from(["94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb".to_string()]);
        let transfers = vec![
            TokenTransferDetails {
                amount: 540059097867,
                ui_amount: 540059.097867,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "B67fRazWFUd4DPgFgLjKXX9dC8vS1UMbXmzYo7YDAqeA".to_string(),
                destination: "9qr6mtX3fELoWGQJyVzHgxuQZptZhmHRMdgZNyGDZkjB".to_string(),
                mint: "2Y6GkQJR93PNL1iYwGcjggoaBRaeTM1p9pC7oCzTpump".to_string(),
                source: "GkcKiF8ku7e54A8NK4UPHW6rmoGfhMeiMHGPpn4yUTkG".to_string(),
            },
            TokenTransferDetails {
                authority: "4sDjn4xpDBzd2QiKKGqmprCxeSLaDygC5oijyLLo6qUX".to_string(),
                destination: "39NaF7ehkzNcxXLq9WZdtQ18RFu1rVxs3oQR1a2safoT".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "GHjM41KiTeTiRR2m42RQF4jSpho4C4KKSx4D1ZX7D3Qb".to_string(),
                amount: 501000002,
                decimals: 9,
                ui_amount: 0.501000002,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            },
            TokenTransferDetails {
                amount: 250001,
                ui_amount: 0.000250001,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "4sDjn4xpDBzd2QiKKGqmprCxeSLaDygC5oijyLLo6qUX".to_string(),
                destination: "94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "GHjM41KiTeTiRR2m42RQF4jSpho4C4KKSx4D1ZX7D3Qb".to_string(),
            },
        ];

        let is_valid =
            is_swap_inner_transfer(&transfers[0], &user_adas, &vaults_adas, Some(&fee_adas));
        assert!(is_valid, "token should be valid");

        let is_valid =
            is_swap_inner_transfer(&transfers[1], &user_adas, &vaults_adas, Some(&fee_adas));
        assert!(is_valid, "wsol ix should be valid");
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn test_f64_to_u64() {
        let supply: u64 = 9999998118661610216;
        let supply_bigdecimal = bigdecimal::BigDecimal::from(supply);
        let actual =
            supply_bigdecimal.div(10_f64.powi(5)).to_f64().expect("failed to convert to f64");
        assert_eq!(actual, 99999981186616.10216);
    }
}
