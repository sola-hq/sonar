use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_raydium_clmm_decoder::instructions::{
    swap::{Swap, SwapInstructionAccounts},
    swap_v2::{SwapV2, SwapV2InstructionAccounts},
    RaydiumClmmInstruction,
};
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Raydium CLMM
pub static RAYDIUM_CLMM_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Raydium CLMM
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Raydium CLMM swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
pub fn get_raydium_clmm_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(RAYDIUM_CLMM_QUOTE_MINTS.clone())
}

impl From<SwapInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SwapInstructionAccounts) -> Self {
        let pair = accounts.pool_state.to_string();
        let user_adas = HashSet::from([
            accounts.input_token_account.to_string(),
            accounts.output_token_account.to_string(),
        ]);
        let vault_adas =
            HashSet::from([accounts.input_vault.to_string(), accounts.output_vault.to_string()]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas,
            fee_adas: None,
            quote_mints: get_raydium_clmm_quote_mints(),
        }
    }
}

impl From<SwapV2InstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SwapV2InstructionAccounts) -> Self {
        let pair = accounts.pool_state.to_string();
        let user_adas = HashSet::from([
            accounts.input_token_account.to_string(),
            accounts.output_token_account.to_string(),
        ]);
        let vault_adas =
            HashSet::from([accounts.input_vault.to_string(), accounts.output_vault.to_string()]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas,
            fee_adas: None,
            quote_mints: get_raydium_clmm_quote_mints(),
        }
    }
}
pub struct RaydiumClmmInstructionProcessor {
    swap_handler: Arc<TokenSwapHandler>,
}

impl RaydiumClmmInstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for RaydiumClmmInstructionProcessor {
    type InputType = InstructionProcessorInputType<RaydiumClmmInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        match &instruction.data {
            RaydiumClmmInstruction::Swap(_e) => {
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
            RaydiumClmmInstruction::SwapV2(_e) => {
                let accounts = SwapV2::arrange_accounts(&instruction.accounts);
                if let Some(accounts) = accounts {
                    let token_swap_accounts = TokenSwapAccounts::from(accounts);
                    self.swap_handler.spawn_swap_instruction(
                        &token_swap_accounts,
                        &meta,
                        &nested_instructions,
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod clmm_tests {
    use super::*;
    use crate::{
        handler::token_swap_handler::filter_swap_transfers,
        test_swaps::{
            get_inner_token_transfers, get_nested_instruction, get_sol_price,
            get_token_swap_handler, TokenTransferDetails,
        },
    };
    use carbon_core::{
        datasource::TransactionUpdate,
        instruction::{DecodedInstruction, InstructionDecoder, NestedInstruction},
        transaction::TransactionMetadata,
    };
    use carbon_raydium_clmm_decoder::RaydiumClmmDecoder;
    use dotenvy::dotenv;

    async fn test_with_clmm_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<RaydiumClmmInstruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = RaydiumClmmDecoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    #[tokio::test]
    async fn test_swap_processor() {
        dotenv().ok();
        let sol_price = get_sol_price().await;
        assert!(sol_price > 0.0);

        let tx_hash = "2mV1jrKN2QMDkKdkLdNNp7iLPpW7xUG21R7NYSTYrPG6hpfQ4KC34b3XMXjc19mA9RsvFzYU5ws25E7aD24EKaf1";
        let outer_index = 2;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_clmm_decoder(tx_hash, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 2);
        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                amount: 170557402,
                ui_amount: 170.557402,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                source: "BwbkRoAAdZ4HHpP6CwEHoxLvGnvrKPi8sJQhHmea2nr4".to_string(),
                destination: "6mK4Pxs6GhwnessH7CvPivqDYauiHZmAdbEFDpXFk9zt".to_string(),
                authority: "8MFMKK2KN6fvkhMiDUtjBjrukYzncUkPDDCiLzabp6ps".to_string(),
            }
        );

        // assert_eq!(
        //     transfers[1],
        //     TokenTransferDetails {
        //         mint: "So11111111111111111111111111111111111111112".to_string(),
        //         amount: 1192089224,
        //         ui_amount: 1.192089224,
        //         decimals: 9,
        //         program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        //         source: "6P4tvbzRY6Bh3MiWDHuLqyHywovsRwRpfskPvyeSoHsz".to_string(),
        //         destination: "2vk4HZjqEY7KKPCgFba7t9Mq2NBWDAGnqY4ksnzcMJCN".to_string(),
        //         authority: "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj".to_string(),
        //     }
        // );

        let accounts = Swap::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let user_adas = HashSet::from([
            accounts.input_token_account.to_string(),
            accounts.output_token_account.to_string(),
        ]);
        let vault_adas =
            HashSet::from([accounts.input_vault.to_string(), accounts.output_vault.to_string()]);
        let token_swap_accounts = TokenSwapAccounts {
            pair: accounts.pool_state.to_string(),
            user_adas,
            vault_adas,
            fee_adas: None,
            quote_mints: get_raydium_clmm_quote_mints(),
        };
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = RaydiumClmmInstructionProcessor::new(token_swap_handler.clone());
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

        tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
    }

    #[tokio::test]
    async fn test_swap_v2_processor() {
        let tx_hash = "65coymtGUzFFxZFvcnaoAxD4MCo6RtkNA5hoZje2az93De5sfnmQP7j5t7AC84H3jFXsBzQHM7kjnZMZVtKfNEjF";
        let outer_index = 3;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_clmm_decoder(tx_hash, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 2);
        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                mint: "HeLp6NuQkmYB4pYWo2zYs22mESHXPQYzXbB8n4V98jwC".to_string(),
                amount: 428375206057,
                ui_amount: 428.375206057,
                decimals: 9,
                program_id: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string(),
                source: "J4JyNJA2V2ADoFZMHnuwUSzWzM1SoETwMtJj4DeHmgKH".to_string(),
                destination: "AimqbbEUThxzK5bhkcjgCCpCb7QN8iPqNvn2qgVE7vat".to_string(),
                authority: "Hq8MmCBFavX2GooSCk9XFp4Whue3wmC3jaZqk1zDgSXx".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                mint: "So11111111111111111111111111111111111111112".to_string(),
                amount: 964026560,
                ui_amount: 0.96402656,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                source: "DsD2zS3Y8GUayzNgg3EZ8wQmvtCheRXzNy2WSgw5rMh8".to_string(),
                destination: "6uoSSkqmEjihppm9erMLDEMSR6YkbKBbNbJRpZXGsaVq".to_string(),
                authority: "8sN9549P3Zn6xpQRqpApN57xzkCh6sJxLwuEjcG2W4Ji".to_string(),
            }
        );

        let accounts = SwapV2::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = RaydiumClmmInstructionProcessor::new(token_swap_handler.clone());
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
    }
}
