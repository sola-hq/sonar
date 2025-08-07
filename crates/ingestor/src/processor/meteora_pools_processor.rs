use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_meteora_pools_decoder::instructions::{
    swap::{Swap, SwapInstructionAccounts},
    MeteoraPoolsProgramInstruction,
};
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Meteora Pools
pub static METEORA_POOLS_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Meteora Pools
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Meteora Pools swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
fn get_meteora_pools_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(METEORA_POOLS_QUOTE_MINTS.clone())
}

impl From<SwapInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SwapInstructionAccounts) -> Self {
        let pair = accounts.pool.to_string();
        let user_adas = HashSet::from([
            accounts.user_source_token.to_string(),      // User Token In
            accounts.user_destination_token.to_string(), // User Token Out
        ]);
        let vaults_adas = HashSet::from([
            accounts.a_token_vault.to_string(), // Reserve X
            accounts.b_token_vault.to_string(), // Reserve Y
        ]);
        let fee_adas = HashSet::from([accounts.protocol_token_fee.to_string()]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas: vaults_adas,
            fee_adas: Some(fee_adas),
            quote_mints: get_meteora_pools_quote_mints(),
        }
    }
}

pub struct MeteoraPoolsInstructionProcessor {
    pub swap_handler: Arc<TokenSwapHandler>,
}

impl MeteoraPoolsInstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for MeteoraPoolsInstructionProcessor {
    type InputType = InstructionProcessorInputType<MeteoraPoolsProgramInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        if let MeteoraPoolsProgramInstruction::Swap(_) = &instruction.data {
            let accounts = Swap::arrange_accounts(&instruction.accounts);
            if let Some(accounts) = accounts {
                let token_swap_accounts = TokenSwapAccounts::from(accounts);
                self.swap_handler.spawn_swap_instruction(
                    &token_swap_accounts,
                    &meta,
                    &nested_instructions,
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod meteora_pools_tests {
    use super::*;
    use crate::{
        handler::token_swap_handler::filter_swap_transfers,
        test_swaps::{
            get_inner_token_transfers, get_nested_instruction, get_token_swap_handler,
            TokenTransferDetails,
        },
    };
    use carbon_core::{
        datasource::TransactionUpdate,
        instruction::{DecodedInstruction, InstructionDecoder, NestedInstruction},
        transaction::TransactionMetadata,
    };
    use carbon_meteora_pools_decoder::MeteoraPoolsDecoder;

    async fn test_with_pools_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<MeteoraPoolsProgramInstruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = MeteoraPoolsDecoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    /// https://solscan.io/tx/3mj1maNypyF6hpHKoPtkTuvM1DTLnpb8H5XP1xeKwCXoxKtH8XX74Jqui9iksGRd5TkYaWvvJRGHHz1THDMcgcV1
    /// #7.2 - Meteora Pools Program: swap
    /// Swap 4.93518 $859.86 WSOL for 4,895,619.144661354 GUILD on Meteora Pools Program (Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB)
    #[tokio::test]
    async fn test_swap_guild_for_sol_processor() {
        let signature = "3mj1maNypyF6hpHKoPtkTuvM1DTLnpb8H5XP1xeKwCXoxKtH8XX74Jqui9iksGRd5TkYaWvvJRGHHz1THDMcgcV1";
        let outer_index = 6;
        let inner_index = Some(1);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_pools_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;
        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 3);

        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "7gwgz9dEg3RMPMafRBJ345xJp7nJZzwHZuZuspCd6o8b".to_string(),
                destination: "Df2bbTJ1SVKKR8THanzzAUUvzrKbpKksoitA9MPFwDim".to_string(),
                source: "5FdsoZWfvQRut5YVnEhjYU1CEAsVGdGnDcb7KXPisbgw".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                decimals: 9,
                amount: 19820000,
                ui_amount: 0.01982
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                authority: "7gwgz9dEg3RMPMafRBJ345xJp7nJZzwHZuZuspCd6o8b".to_string(),
                destination: "HZeLxbZ9uHtSpwZC3LBr4Nubd14iHwz7bRSghRZf5VCG".to_string(),
                source: "5FdsoZWfvQRut5YVnEhjYU1CEAsVGdGnDcb7KXPisbgw".to_string(),
                decimals: 9,
                amount: 4935180000,
                ui_amount: 4.93518
            }
        );

        assert_eq!(
            transfers[2],
            TokenTransferDetails {
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                mint: "sYGMwFdX53FrB2ajiB3TPDrHwnLUk4pQ7mCfFxWphoX".to_string(),
                authority: "2dDk39eyD9Vg64Sd2mSKR4jrABhS5xcBUCxy4eY5fnZR".to_string(),
                destination: "Az7ModQzX8KhBNwymibMSHm3w2LkLaRhGVmNFmBzUfP8".to_string(),
                source: "Dq9jdRo94L8RExKg94zkYDZfpqCfeB7g1JjvS5fydiZU".to_string(),
                decimals: 9,
                amount: 4895619144661354,
                ui_amount: 4895619.144661354
            }
        );
        let accounts = Swap::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = MeteoraPoolsInstructionProcessor::new(token_swap_handler.clone());
        processor
            .process(
                (
                    nested_instruction.metadata.clone(),
                    instruction,
                    nested_instruction.inner_instructions.clone(),
                    nested_instruction.instruction.clone(),
                ),
                Arc::new(MetricsCollection::new(vec![])),
            )
            .await
            .expect("Failed to process instruction");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
