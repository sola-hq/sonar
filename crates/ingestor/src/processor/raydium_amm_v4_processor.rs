use crate::{
    constants::{Dexes, USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_raydium_amm_v4_decoder::instructions::{
    initialize2, initialize2::Initialize2, swap_base_in, swap_base_in::SwapBaseIn, swap_base_out,
    swap_base_out::SwapBaseOut, RaydiumAmmV4Instruction,
};
use chrono::Utc;
use solana_pubkey::Pubkey;
use sonar_db::models::NewPoolEvent;
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Raydium AMM V4
pub static RAYDIUM_AMM_V4_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Raydium AMM V4
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Raydium AMM V4 swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
fn get_raydium_amm_v4_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(RAYDIUM_AMM_V4_QUOTE_MINTS.clone())
}

fn create_token_swap_accounts(
    amm: &Pubkey,
    user_source: &Pubkey,
    user_destination: &Pubkey,
    pool_coin: &Pubkey,
    pool_pc: &Pubkey,
) -> TokenSwapAccounts {
    let pair = amm.to_string();
    let user_adas = HashSet::from([user_source.to_string(), user_destination.to_string()]);
    let vault_adas = HashSet::from([pool_coin.to_string(), pool_pc.to_string()]);

    TokenSwapAccounts {
        pair,
        user_adas,
        vault_adas,
        fee_adas: None,
        quote_mints: get_raydium_amm_v4_quote_mints(),
    }
}

pub fn get_new_pool_event(
    accounts: initialize2::Initialize2InstructionAccounts,
    timestamp: u64,
) -> NewPoolEvent {
    NewPoolEvent {
        dex: Dexes::RaydiumAmmV4.to_string(),
        token_a_mint: accounts.coin_mint.to_string(),
        token_b_mint: accounts.pc_mint.to_string(),
        pool: accounts.amm.to_string(),
        timestamp,
    }
}

impl From<swap_base_in::SwapBaseInInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: swap_base_in::SwapBaseInInstructionAccounts) -> Self {
        create_token_swap_accounts(
            &accounts.amm,
            &accounts.user_source_token_account,
            &accounts.user_destination_token_account,
            &accounts.pool_coin_token_account,
            &accounts.pool_pc_token_account,
        )
    }
}

impl From<swap_base_out::SwapBaseOutInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: swap_base_out::SwapBaseOutInstructionAccounts) -> Self {
        create_token_swap_accounts(
            &accounts.amm,
            &accounts.user_source_token_account,
            &accounts.user_destination_token_account,
            &accounts.pool_coin_token_account,
            &accounts.pool_pc_token_account,
        )
    }
}

pub struct RaydiumAmmV4InstructionProcessor {
    swap_handler: Arc<TokenSwapHandler>,
}

impl RaydiumAmmV4InstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for RaydiumAmmV4InstructionProcessor {
    type InputType = InstructionProcessorInputType<RaydiumAmmV4Instruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        match &instruction.data {
            RaydiumAmmV4Instruction::SwapBaseIn(_) => {
                let accounts = SwapBaseIn::arrange_accounts(&instruction.accounts);
                if let Some(accounts) = accounts {
                    let token_swap_accounts = TokenSwapAccounts::from(accounts);
                    self.swap_handler.spawn_swap_instruction(
                        &token_swap_accounts,
                        &meta,
                        &nested_instructions,
                    );
                }
            }
            RaydiumAmmV4Instruction::SwapBaseOut(_) => {
                let accounts = SwapBaseOut::arrange_accounts(&instruction.accounts);
                if let Some(accounts) = accounts {
                    let token_swap_accounts = TokenSwapAccounts::from(accounts);
                    self.swap_handler.spawn_swap_instruction(
                        &token_swap_accounts,
                        &meta,
                        &nested_instructions,
                    );
                }
            }
            RaydiumAmmV4Instruction::Initialize2(_) => {
                let accounts = Initialize2::arrange_accounts(&instruction.accounts);
                if let Some(accounts) = accounts {
                    let block_time =
                        meta.transaction_metadata.block_time.unwrap_or(Utc::now().timestamp())
                            as u64;
                    let new_pool_event = get_new_pool_event(accounts, block_time);
                    self.swap_handler.spawn_new_pool_instruction(&meta, new_pool_event);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod amm_v4_tests {
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
    use carbon_raydium_amm_v4_decoder::RaydiumAmmV4Decoder;

    async fn test_with_amm_v4_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<RaydiumAmmV4Instruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = RaydiumAmmV4Decoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    #[tokio::test]
    async fn test_spawn_swap_processor() {
        let tx_hash = "31pB39KowUTdDSjXhzCYi7QxVSWSM4ZijaSWAkCduWUUR6GuGrWwVBbcXLLdJnVLrWbQaV7YFL2SigBXRatGfnji";
        let outer_index = 3;
        let inner_index = Some(1);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_amm_v4_decoder(tx_hash, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 2);
        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                mint: "AsyfR3e5JcPqWot4H5MMhQUm7DZ4zwQrcp2zbB7vpump".to_string(),
                amount: 279274681533,
                ui_amount: 279274.681533,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                source: "3oV3EFEp6GUTt8cn3swj1oQXhmeuRyKv9cEzpSVZga5K".to_string(),
                destination: "HqDtzxBsHHhmTHbzmUk5aJkAZE8iGf6KKeeYrh4mVCc3".to_string(),
                authority: "6LXutJvKUw8Q5ue2gCgKHQdAN4suWW8awzFVC6XCguFx".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                mint: "So11111111111111111111111111111111111111112".to_string(),
                amount: 8569783440,
                ui_amount: 8.56978344,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                source: "6M2KAV658rer6g2L7tAAQtXK7f1GmrbG7ycW14gHdK5U".to_string(),
                destination: "BuqEDKUwyAotZuK37V4JYEykZVKY8qo1zKbpfU9gkJMo".to_string(),
                authority: "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1".to_string(),
            }
        );

        let accounts = SwapBaseIn::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let user_adas = HashSet::from([
            accounts.user_source_token_account.to_string(),
            accounts.user_destination_token_account.to_string(),
        ]);
        let vaults_adas = HashSet::from([
            accounts.pool_coin_token_account.to_string(),
            accounts.pool_pc_token_account.to_string(),
        ]);

        let token_swap_accounts = TokenSwapAccounts {
            pair: "".to_string(),
            user_adas,
            vault_adas: vaults_adas,
            fee_adas: None,
            quote_mints: get_raydium_amm_v4_quote_mints(),
        };
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = RaydiumAmmV4InstructionProcessor::new(token_swap_handler.clone());
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
