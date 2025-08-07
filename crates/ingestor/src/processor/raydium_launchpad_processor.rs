use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_raydium_launchpad_decoder::instructions::{
    sell_exact_in::{SellExactIn, SellExactInInstructionAccounts},
    sell_exact_out::{SellExactOut, SellExactOutInstructionAccounts},
    RaydiumLaunchpadInstruction,
};
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Raydium Launchpad
pub static RAYDIUM_LAUNCHPAD_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Raydium Launchpad
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Raydium Launchpad swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
pub fn get_raydium_launchpad_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(RAYDIUM_LAUNCHPAD_QUOTE_MINTS.clone())
}

impl From<SellExactInInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SellExactInInstructionAccounts) -> Self {
        let pair = accounts.pool_state.to_string();
        let user_adas = HashSet::from([
            accounts.user_base_token.to_string(),
            accounts.user_quote_token.to_string(),
        ]);
        let vault_adas =
            HashSet::from([accounts.base_vault.to_string(), accounts.quote_vault.to_string()]);

        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas,
            fee_adas: None,
            quote_mints: get_raydium_launchpad_quote_mints(),
        }
    }
}

impl From<SellExactOutInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SellExactOutInstructionAccounts) -> Self {
        let pair = accounts.pool_state.to_string();
        let user_adas = HashSet::from([
            accounts.user_base_token.to_string(),
            accounts.user_quote_token.to_string(),
        ]);
        let vault_adas =
            HashSet::from([accounts.base_vault.to_string(), accounts.quote_vault.to_string()]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas,
            fee_adas: None,
            quote_mints: get_raydium_launchpad_quote_mints().clone(),
        }
    }
}

pub struct RaydiumLaunchpadInstructionProcessor {
    swap_handler: Arc<TokenSwapHandler>,
}

impl RaydiumLaunchpadInstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for RaydiumLaunchpadInstructionProcessor {
    type InputType = InstructionProcessorInputType<RaydiumLaunchpadInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        match &instruction.data {
            RaydiumLaunchpadInstruction::SellExactIn(_) => {
                let accounts = SellExactIn::arrange_accounts(&instruction.accounts);
                if let Some(accounts) = accounts {
                    let token_swap_accounts = TokenSwapAccounts::from(accounts);
                    self.swap_handler.spawn_swap_instruction(
                        &token_swap_accounts,
                        &meta,
                        &nested_instructions,
                    );
                }
            }
            RaydiumLaunchpadInstruction::SellExactOut(_) => {
                let accounts = SellExactOut::arrange_accounts(&instruction.accounts);
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
mod cpmm_tests {
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
    use carbon_raydium_launchpad_decoder::RaydiumLaunchpadDecoder;

    async fn test_with_launchpad_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<RaydiumLaunchpadInstruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = RaydiumLaunchpadDecoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    /// https://solscan.io/tx/5uAcUFZbwbA2UuGFcaDZnMypCjGvTxrRwJzwUuUt8wm2RGuv4JzQ8hcCgVvphvsWsAh3tD8zN5Js7q7mSPN75Ecd
    /// #5 - Raydium Launchpad: buy_exact_in
    /// Swap 0.1 $17.31 image WSOL for 1,032,026.34136 MoM on Raydium Launchpad
    #[tokio::test]
    async fn test_sell_exact_in_processor() {
        let tx_hash = "5uAcUFZbwbA2UuGFcaDZnMypCjGvTxrRwJzwUuUt8wm2RGuv4JzQ8hcCgVvphvsWsAh3tD8zN5Js7q7mSPN75Ecd";
        let outer_index = 4;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_launchpad_decoder(tx_hash, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);

        assert_eq!(transfers.len(), 2);

        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                amount: 100000000,
                ui_amount: 0.1,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "81dToCK1gB3Lb4m3xcfrn8Go1u7M1kpeNGaUpgvg1e4W".to_string(),
                destination: "CXf6k7BjP7DYGmkT6CwxTtjeNB2hJLB7CYPPgob3uZbq".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "9juawE37ibJVEjvkRdR62oaiEcVvtFdmttK5tEphP45H".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                amount: 1032026341360,
                ui_amount: 1032026.34136,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh".to_string(),
                destination: "6jBMeoLH78Qy5hjAjPaKkSCegKeadMytzQrPsKHazFTz".to_string(),
                mint: "24YqgtkwPMmfMHNfvErYLomsuw1R4CWv5V9iaC22bonk".to_string(),
                source: "Hh82CVt5CAvpj3DhotUgxTrYDCPxwLAsgcDZcFbCuoB4".to_string(),
            }
        );

        let accounts = SellExactIn::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = RaydiumLaunchpadInstructionProcessor::new(token_swap_handler.clone());
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
