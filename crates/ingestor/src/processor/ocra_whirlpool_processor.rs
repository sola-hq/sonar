use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_orca_whirlpool_decoder::instructions::{
    swap::{Swap, SwapInstructionAccounts},
    swap_v2::{SwapV2, SwapV2InstructionAccounts},
    OrcaWhirlpoolInstruction,
};
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Orca Whirlpool
pub static ORCA_WHIRLPOOL_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Orca Whirlpool
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Orca Whirlpool swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
fn get_orca_whirlpool_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(ORCA_WHIRLPOOL_QUOTE_MINTS.clone())
}

impl From<SwapInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SwapInstructionAccounts) -> Self {
        let pair = accounts.whirlpool.to_string();
        let user_adas = HashSet::from([
            accounts.token_owner_account_a.to_string(),
            accounts.token_owner_account_b.to_string(),
        ]);
        let vaults_adas =
            HashSet::from([accounts.token_vault_a.to_string(), accounts.token_vault_b.to_string()]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas: vaults_adas,
            fee_adas: None,
            quote_mints: get_orca_whirlpool_quote_mints(),
        }
    }
}

impl From<SwapV2InstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SwapV2InstructionAccounts) -> Self {
        let pair = accounts.whirlpool.to_string();
        let user_adas = HashSet::from([
            accounts.token_owner_account_a.to_string(),
            accounts.token_owner_account_b.to_string(),
        ]);
        let vault_adas =
            HashSet::from([accounts.token_vault_a.to_string(), accounts.token_vault_b.to_string()]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas,
            fee_adas: None,
            quote_mints: get_orca_whirlpool_quote_mints(),
        }
    }
}

pub struct OcraWhirlpoolInstructionProcessor {
    swap_handler: Arc<TokenSwapHandler>,
}

impl OcraWhirlpoolInstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for OcraWhirlpoolInstructionProcessor {
    type InputType = InstructionProcessorInputType<OrcaWhirlpoolInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        match &instruction.data {
            OrcaWhirlpoolInstruction::Swap(_) => {
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
            OrcaWhirlpoolInstruction::SwapV2(_) => {
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
mod orca_whirlpool_tests {
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
    use carbon_orca_whirlpool_decoder::OrcaWhirlpoolDecoder;

    async fn test_with_whirlpool_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<OrcaWhirlpoolInstruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = OrcaWhirlpoolDecoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    /// https://solscan.io/tx/3ankeujUXU4EPjcJXFdNrn4nqGVati1KpMntYfTpgGhboxywLVb2oYpG9BStMwGojjvGSfNff4Zar8tPqX9ifJMP
    /// #2 - Whirlpools Program: swap
    #[tokio::test]
    async fn test_swap_base_output_processor() {
        let signature = "3ankeujUXU4EPjcJXFdNrn4nqGVati1KpMntYfTpgGhboxywLVb2oYpG9BStMwGojjvGSfNff4Zar8tPqX9ifJMP";
        let outer_index = 1;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_whirlpool_decoder(signature, outer_index, inner_index).await;
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
                amount: 1961878075,
                ui_amount: 1961.878075,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "MfDuWeqSHEqTFVYZ7LoexgAK9dxk7cy4DFJWjWMGVWa".to_string(),
                destination: "79Lv5tG6n74sRJFLjrXxwqBdNmFv8ERYQZ1WiSUbCDU4".to_string(),
                mint: "61V8vBaqAGMpgDQi4JcAwo1dmBGHsyhzodcPqnEVpump".to_string(),
                source: "3g4yFngFJyQppCFcaD2sbPe4HdLzQiS64MfPSPLK5iN5".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                amount: 1241037050,
                ui_amount: 1.24103705,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "57mP5WoNrg3uiGFUdoeYr2CPUZak1L2ZgFtyFwoT7K6G".to_string(),
                destination: "CTyFguG69kwYrzk24P3UuBvY1rR5atu9kf2S6XEwAU8X".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "CcwLMXxRLaaf1biHSaXCckQB85xyq3U7GRo3iiqCV74H".to_string(),
            }
        );

        let mut processor = OcraWhirlpoolInstructionProcessor::new(token_swap_handler.clone());
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

    /// https://solscan.io/tx/537dMaSgP8nA3Xf1y36x3pCzysEVwBRWrrMNJpefMRR5MrB6ATjDyGJzwu8uVR9ku8Vi65wRdHrHBGQDQNMvXdtq
    /// #2 - Whirlpools Program: swap
    /// Swap 3.85937738 WSOL for 499.598215 USDC on Whirlpools Program
    #[tokio::test]
    async fn test_wsol_for_usdc() {
        let signature = "537dMaSgP8nA3Xf1y36x3pCzysEVwBRWrrMNJpefMRR5MrB6ATjDyGJzwu8uVR9ku8Vi65wRdHrHBGQDQNMvXdtq";
        let outer_index = 2;
        let inner_index = Some(0);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_whirlpool_decoder(signature, outer_index, inner_index).await;
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
                amount: 3859377380,
                ui_amount: 3.85937738,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "YubQzu18FDqJRyNfG8JqHmsdbxhnoQqcKUHBdUkN6tP".to_string(),
                destination: "EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "7jcTwYAN2Ai7C3hjfa2hkRsd9B3BiFXY3kniXD4eJucP".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                amount: 499598215,
                ui_amount: 499.598215,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string(),
                destination: "GpKb5wb4A81kGzsy8Wf5vq5eNmtW7vKTKuXgt6Yg6JP2".to_string(),
                mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                source: "2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP".to_string(),
            }
        );

        let mut processor = OcraWhirlpoolInstructionProcessor::new(token_swap_handler.clone());
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
