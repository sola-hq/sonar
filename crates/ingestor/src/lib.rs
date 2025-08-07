pub mod constants;
pub mod datasource;
pub mod decoder;
pub mod handler;
pub mod metrics;
pub mod processor;

pub use handler::{
    get_inner_token_transfers, get_swap_event_with_token_transfer_details,
    process_token_swap_instruction, TokenSwapAccounts, TokenSwapHandler,
};

pub mod prelude {
    pub use crate::datasource::{
        block::make_block_crawler_datasource, build_pipeline, geyser::make_geyser_datasource,
        helius::make_helius_ws_datasource, rpc::make_rpc_client,
        tx::make_transaction_crawler_datasource, ws::make_ws_datasource,
    };
}

#[cfg(test)]
pub mod test_swaps {
    pub use crate::{
        decoder::TokenTransferDetails,
        handler::{get_inner_token_transfers, TokenSwapHandler},
    };
    use crate::{metrics::NodeMetrics, prelude::make_rpc_client};
    use anyhow::{anyhow, Result};
    use carbon_core::{
        datasource::TransactionUpdate,
        instruction::{NestedInstruction, NestedInstructions},
        transaction::TransactionMetadata,
        transformers::{
            extract_instructions_with_metadata, transaction_metadata_from_original_meta,
        },
    };
    use dotenvy::dotenv;
    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_commitment_config::CommitmentConfig;
    use solana_signature::Signature;
    use solana_transaction_status::UiTransactionEncoding;
    use sonar_db::{
        make_db_from_env, make_kv_store_from_env, make_message_queue_from_env, Database, KvStore,
        MessageQueue,
    };
    use sonar_sol_price::SolPriceCache;
    use std::{str::FromStr, sync::Arc};

    pub async fn get_storages() -> (Arc<KvStore>, Arc<MessageQueue>, Arc<Database>) {
        let kv_store = make_kv_store_from_env().await.expect("Failed to make kv store");
        let message_queue =
            make_message_queue_from_env().await.expect("Failed to make message queue");
        let db = make_db_from_env().await.expect("Failed to make db");
        (Arc::new(kv_store), Arc::new(message_queue), Arc::new(db))
    }

    pub async fn get_sol_price() -> f64 {
        let (kv_store, message_queue, _db) = get_storages().await;
        let price_cache = SolPriceCache::new(Some(kv_store.clone()), Some(message_queue.clone()));
        let price_cache = Arc::new(price_cache);
        price_cache.get_price().await
    }

    pub async fn get_token_swap_handler() -> Arc<TokenSwapHandler> {
        let (kv_store, message_queue, db) = get_storages().await;
        let metrics = Arc::new(NodeMetrics::new());
        Arc::new(TokenSwapHandler::new(kv_store, message_queue, db, metrics))
    }

    pub async fn get_transaction_data(
        tx_hash: &str,
    ) -> Result<(Signature, Box<TransactionUpdate>, Box<TransactionMetadata>)> {
        dotenv().ok();
        let signature = Signature::from_str(tx_hash).expect("Failed to parse signature");
        let rpc_client = make_rpc_client();
        let encoded_transaction = rpc_client
            .get_transaction_with_config(
                &signature,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Binary),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
            .await
            .expect("Failed to get transaction");

        let transaction = encoded_transaction.transaction;

        let meta_original = if let Some(meta) = transaction.clone().meta {
            meta
        } else {
            return Err(anyhow!("Meta is malformed for transaction: {:?}", signature));
        };

        if meta_original.status.is_err() {
            return Err(anyhow!("Transaction failed: {:?}", signature));
        }

        let decoded_transaction = transaction
            .transaction
            .decode()
            .ok_or_else(|| anyhow!("Failed to decode transaction"))?;

        let meta_needed = transaction_metadata_from_original_meta(meta_original)
            .map_err(|e| anyhow!("Error getting metadata: {}", e))?;

        let transaction_update = Box::new(TransactionUpdate {
            signature,
            transaction: decoded_transaction.clone(),
            meta: meta_needed,
            is_vote: false,
            slot: encoded_transaction.slot,
            block_time: encoded_transaction.block_time,
            block_hash: None,
        });

        let transaction_metadata: TransactionMetadata = (*transaction_update)
            .clone()
            .try_into()
            .expect("Failed to convert transaction update to transaction metadata.");

        Ok((signature, transaction_update, Box::new(transaction_metadata)))
    }

    pub async fn get_nested_instruction(
        tx_hash: &str,
        outer_idx: usize,
        inner_idx: Option<usize>,
    ) -> Result<(NestedInstruction, Box<TransactionUpdate>, Box<TransactionMetadata>)> {
        let (_, transaction_update, transaction_metadata) =
            get_transaction_data(tx_hash).await.expect("Failed to get transaction data");
        let nested_instructions = extract_nested_instructions(&transaction_update)
            .expect("Failed to extract nested instructions");
        if outer_idx >= nested_instructions.len() {
            return Err(anyhow!("Outer index out of bounds"));
        }
        let mut nested_instruction = nested_instructions[outer_idx].clone();
        println!(
            "nested_inner_instructions {:?}",
            nested_instruction
                .inner_instructions
                .iter()
                .map(|i| i.instruction.program_id.to_string())
                .collect::<Vec<String>>()
        );
        if let Some(inner_idx) = inner_idx {
            println!(
                "inner_instructions {:?}",
                nested_instructions
                    .iter()
                    .map(|i| i
                        .inner_instructions
                        .iter()
                        .map(|j| j.instruction.program_id.to_string())
                        .collect::<Vec<String>>()
                        .join(", "))
                    .collect::<Vec<String>>()
            );
            nested_instruction = nested_instruction.inner_instructions[inner_idx].clone()
        }
        Ok((nested_instruction, transaction_update, transaction_metadata))
    }

    pub fn extract_nested_instructions(
        transaction_update: &TransactionUpdate,
    ) -> Result<NestedInstructions> {
        let transaction_metadata: TransactionMetadata = transaction_update
            .clone()
            .try_into()
            .map_err(|e| anyhow!("Failed to convert transaction update: {}", e))?;
        let transaction_metadata = Arc::new(transaction_metadata);
        let instructions_with_metadata =
            extract_instructions_with_metadata(&transaction_metadata, transaction_update)
                .map_err(|e| anyhow!("Failed to extract instructions: {}", e))?;

        Ok(instructions_with_metadata.into())
    }
}
