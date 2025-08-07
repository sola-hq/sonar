use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_meteora_dlmm_decoder::instructions::{
    swap::{Swap, SwapInstructionAccounts},
    MeteoraDlmmInstruction,
};
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Meteora DLMM
pub static METEORA_DLMM_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Meteora DLMM
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Meteora DLMM swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
pub fn get_meteora_dlmm_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(METEORA_DLMM_QUOTE_MINTS.clone())
}

impl From<SwapInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SwapInstructionAccounts) -> Self {
        let pair = accounts.lb_pair.to_string();
        let user_adas = HashSet::from([
            accounts.user_token_in.to_string(),  // User Token In
            accounts.user_token_out.to_string(), // User Token Out
        ]);
        let vaults_adas = HashSet::from([
            accounts.reserve_x.to_string(), // Reserve X
            accounts.reserve_y.to_string(), // Reserve Y
        ]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas: vaults_adas,
            fee_adas: None,
            quote_mints: get_meteora_dlmm_quote_mints(),
        }
    }
}

pub struct MeteoraDlmmInstructionProcessor {
    swap_handler: Arc<TokenSwapHandler>,
}

impl MeteoraDlmmInstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for MeteoraDlmmInstructionProcessor {
    type InputType = InstructionProcessorInputType<MeteoraDlmmInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        if let MeteoraDlmmInstruction::Swap(_) = &instruction.data {
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
mod meteora_dlmm_tests {
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
    use carbon_meteora_dlmm_decoder::MeteoraDlmmDecoder;

    async fn test_with_dlmm_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<MeteoraDlmmInstruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = MeteoraDlmmDecoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    /// https://solscan.io/tx/3m4LERWUekW7im8rgu8QgpSJA8a9yEYL3gDvorbd5YpkXarrL3PGoVmyFyQzd1Pw9oZiQy2LPUjaG8Xr4p433kwn
    /// #3.6 - Meteora DLMM Program: swap
    #[tokio::test]
    async fn test_swap_base_output_processor() {
        let signature = "3m4LERWUekW7im8rgu8QgpSJA8a9yEYL3gDvorbd5YpkXarrL3PGoVmyFyQzd1Pw9oZiQy2LPUjaG8Xr4p433kwn";
        let outer_index = 2;
        let inner_index = Some(3);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_dlmm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 2);

        let accounts = Swap::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let filtered_transfers = filter_swap_transfers(&transfers, &token_swap_accounts);

        assert_eq!(filtered_transfers.len(), 2);

        assert_eq!(
            filtered_transfers[0],
            TokenTransferDetails {
                amount: 24000000000,
                ui_amount: 24000.0,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB".to_string(),
                destination: "CMVrNeYhZnqdbZfQuijgcNvCfvTJN2WKvKSnt2q3HT6N".to_string(),
                mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_string(),
                source: "89YMNsMDmHeMhT3BiDTcryRuxWSn24B31Gf5H9N2Z8Zu".to_string(),
            }
        );

        assert_eq!(
            filtered_transfers[1],
            TokenTransferDetails {
                amount: 65256388526,
                ui_amount: 65.256388526,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "6wJ7W3oHj7ex6MVFp2o26NSof3aey7U8Brs8E371WCXA".to_string(),
                destination: "7x4VcEX8aLd3kFsNWULTp1qFgVtDwyWSxpTGQkoMM6XX".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "5EfbkfLpaz9mHeTN6FnhtN8DTdMGZDRURYcsQ1f1Utg6".to_string(),
            }
        );

        let mut processor = MeteoraDlmmInstructionProcessor::new(token_swap_handler.clone());
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

    /// https://solscan.io/tx/5izpNhE1yRru7WuBMV7nWz4DrXvobUWvWuAcTAPgL1tBComtbqQrF6oSbTnhuyv2k4SPX9xNXEpDnunD2eSM1yen
    /// #3.6 - Meteora DLMM Program: swap
    /// Swap 200 USDC for 18.143267 $199.94 TRUMP
    #[tokio::test]
    async fn test_swap_usdc_output_processor() {
        let signature = "5izpNhE1yRru7WuBMV7nWz4DrXvobUWvWuAcTAPgL1tBComtbqQrF6oSbTnhuyv2k4SPX9xNXEpDnunD2eSM1yen";
        let outer_index = 2;
        let inner_index = Some(0);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_dlmm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 2);

        let accounts = Swap::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let filtered_transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(filtered_transfers.len(), 2);

        assert_eq!(
            filtered_transfers[0],
            TokenTransferDetails {
                amount: 200000000,
                ui_amount: 200.0,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "ELyMgUgdWnxSvf4TaWPhZwQGuiseKKKvYiZVKnZeu59N".to_string(),
                destination: "81BadRGfaHFpAmuXpJ65k8tYtUWsZ54EFSmsVo1rbDTV".to_string(),
                mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                source: "55Gc1PsB3MJdcdtCtyU6WqJwTByfWtmRoJfnGZSXSwRM".to_string(),
            }
        );

        assert_eq!(
            filtered_transfers[1],
            TokenTransferDetails {
                amount: 18143267,
                ui_amount: 18.143267,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "9d9mb8kooFfaD3SctgZtkxQypkshx6ezhbKio89ixyy2".to_string(),
                destination: "DTARjZvq6BoWMr2r3ejEvC3opDxLqa12bLq2FYhPwzxw".to_string(),
                mint: "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN".to_string(),
                source: "AK93dERw7MJsGFBUPfV1bkXzDviJZM1K6vg2yGDugk7L".to_string(),
            }
        );

        let mut processor = MeteoraDlmmInstructionProcessor::new(token_swap_handler.clone());
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

    /// https://solscan.io/tx/3iJi5GiGSbhFyu7c7B2MyQU43xtR5krM7bD8pzoL6hJLVRgoqQsKaBXsUDTrvnWrFYsKeZBdDabRVo1d8X2x95YY
    /// #3.1 - Meteora DLMM Program: swap
    /// Swap 1.949327 image USDC for 0.015135932 $2.1175 image WSOL
    #[tokio::test]
    async fn test_swap_usdc_for_sol_processor() {
        let signature = "3iJi5GiGSbhFyu7c7B2MyQU43xtR5krM7bD8pzoL6hJLVRgoqQsKaBXsUDTrvnWrFYsKeZBdDabRVo1d8X2x95YY";
        let outer_index = 2;
        let inner_index = Some(0);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_dlmm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 2);

        let accounts = Swap::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);
        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                source: "EAPyEChWLxDSft76CNTEVQdPnp8c1dPgCdsMchXgJvTC".to_string(),
                destination: "CoaxzEh8p5YyGLcj36Eo3cUThVJxeKCs7qvLAGDYwBcz".to_string(),
                mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                authority: "8L2y55D11k63CAftvW7uMM2mBhtMxLoLnivG9uY2bt8j".to_string(),
                decimals: 6,
                amount: 1949327,
                ui_amount: 1.949327
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                source: "EYj9xKw6ZszwpyNibHY7JD5o3QgTVrSdcBp1fMJhrR9o".to_string(),
                destination: "3ZuE2darhRutPG8w2bMcq5chG4CgFBEEs4QHJCYyWLqF".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                authority: "5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6".to_string(),
                decimals: 9,
                amount: 15135932,
                ui_amount: 0.015135932
            }
        );

        let mut processor = MeteoraDlmmInstructionProcessor::new(token_swap_handler.clone());
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
