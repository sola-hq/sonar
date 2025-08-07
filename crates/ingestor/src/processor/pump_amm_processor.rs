use crate::{
    constants::{USDC_MINT_KEY_STR, USDT_MINT_KEY_STR, WSOL_MINT_KEY_STR},
    TokenSwapAccounts, TokenSwapHandler,
};
use carbon_core::{
    deserialize::ArrangeAccounts, error::CarbonResult, instruction::InstructionProcessorInputType,
    metrics::MetricsCollection, processor::Processor,
};
use carbon_pump_swap_decoder::instructions::{
    buy::{Buy, BuyInstructionAccounts},
    sell::{Sell, SellInstructionAccounts},
    PumpSwapInstruction,
};
use std::{collections::HashSet, sync::Arc, sync::LazyLock};

/// A set of quote mints supported by Pump.fun AMM
pub static PUMP_AMM_QUOTE_MINTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    HashSet::from([
        USDC_MINT_KEY_STR.to_string(),
        USDT_MINT_KEY_STR.to_string(),
        WSOL_MINT_KEY_STR.to_string(),
    ])
});

/// Returns the set of quote mints supported by Pump.fun AMM
///
/// This function provides access to the predefined set of token mints
/// that are commonly used as quote tokens in Pump.fun AMM swaps.
/// The set includes USDC, USDT, and WSOL (Wrapped SOL).
fn get_pump_amm_quote_mints() -> Arc<HashSet<String>> {
    Arc::new(PUMP_AMM_QUOTE_MINTS.clone())
}

// Buy token account
impl From<BuyInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: BuyInstructionAccounts) -> Self {
        let pair = accounts.pool.to_string();
        let user_adas = HashSet::from([
            accounts.user_base_token_account.to_string(),
            accounts.user_quote_token_account.to_string(),
        ]);
        let vaults_adas = HashSet::from([
            accounts.pool_base_token_account.to_string(),
            accounts.pool_quote_token_account.to_string(),
        ]);
        let fee_adas = HashSet::from([
            accounts.protocol_fee_recipient.to_string(),
            accounts.protocol_fee_recipient_token_account.to_string(),
        ]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas: vaults_adas,
            fee_adas: Some(fee_adas),
            quote_mints: get_pump_amm_quote_mints(),
        }
    }
}

// Sell token account
impl From<SellInstructionAccounts> for TokenSwapAccounts {
    fn from(accounts: SellInstructionAccounts) -> Self {
        let pair = accounts.pool.to_string();
        let user_adas = HashSet::from([
            accounts.user_base_token_account.to_string(),
            accounts.user_quote_token_account.to_string(),
        ]);
        let vaults_adas = HashSet::from([
            accounts.pool_base_token_account.to_string(),
            accounts.pool_quote_token_account.to_string(),
        ]);
        let fee_adas = HashSet::from([
            accounts.protocol_fee_recipient.to_string(),
            accounts.protocol_fee_recipient_token_account.to_string(),
        ]);
        TokenSwapAccounts {
            pair,
            user_adas,
            vault_adas: vaults_adas,
            fee_adas: Some(fee_adas),
            quote_mints: get_pump_amm_quote_mints(),
        }
    }
}

pub struct PumpAmmInstructionProcessor {
    swap_handler: Arc<TokenSwapHandler>,
}

impl PumpAmmInstructionProcessor {
    pub fn new(swap_handler: Arc<TokenSwapHandler>) -> Self {
        Self { swap_handler }
    }
}

#[async_trait::async_trait]
impl Processor for PumpAmmInstructionProcessor {
    type InputType = InstructionProcessorInputType<PumpSwapInstruction>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, instruction, nested_instructions, _) = data;
        if let PumpSwapInstruction::Buy(_) = &instruction.data {
            let accounts = Buy::arrange_accounts(&instruction.accounts);
            if let Some(accounts) = accounts {
                let token_swap_accounts = TokenSwapAccounts::from(accounts);
                self.swap_handler.spawn_swap_instruction(
                    &token_swap_accounts,
                    &meta,
                    &nested_instructions,
                );
            }
        }
        if let PumpSwapInstruction::Sell(_) = &instruction.data {
            let accounts = Sell::arrange_accounts(&instruction.accounts);
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
mod pump_amm_tests {
    use super::*;
    use crate::{
        handler::token_swap_handler::filter_swap_transfers,
        processor::PumpAmmInstructionProcessor,
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
    use carbon_pump_swap_decoder::PumpSwapDecoder;

    async fn test_with_amm_decoder(
        tx_hash: &str,
        outer_index: usize,
        inner_index: Option<usize>,
    ) -> (
        NestedInstruction,
        Option<DecodedInstruction<PumpSwapInstruction>>,
        Box<TransactionUpdate>,
        Box<TransactionMetadata>,
    ) {
        let (nested_instruction, transaction_update, transaction_metadata) =
            get_nested_instruction(tx_hash, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoder = PumpSwapDecoder;
        let instruction = decoder.decode_instruction(&nested_instruction.instruction);
        (nested_instruction, instruction, transaction_update, transaction_metadata)
    }

    /// https://solscan.io/tx/3G7iGWpatj5vjPRmsxRsYh3N6B1WkiBX77u8yizPVcGZkqytdT6UYeCfsHan816sRH3jYpG45FRL3GLywud7CpbT
    /// Swap 2,523 DWH for 0.007229486 $0.9411 WSOL On Pump.fun AMM
    #[tokio::test]
    async fn test_sell_processor() {
        let signature = "3G7iGWpatj5vjPRmsxRsYh3N6B1WkiBX77u8yizPVcGZkqytdT6UYeCfsHan816sRH3jYpG45FRL3GLywud7CpbT";
        let outer_index = 0;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_amm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 3);

        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                amount: 2523000000,
                ui_amount: 2523.0,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "G2gUder2Y934cm8ufSQxjbhjrfJsiBBAox1jgLqEDx75".to_string(),
                destination: "GHs3Cs9J6NoX79Nr2KvR1Nnzm82R34Jmqh1A8Bb84zgc".to_string(),
                mint: "2WZuixz3wohXbib7Ze2gRjVeGeESiMw9hsizDwbjM4YK".to_string(),
                source: "yAcYcbC9Qr9SBpeG9SbT1zAEFwHd8j6EFFWomjQjVtn".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                amount: 7229486,
                ui_amount: 0.007229486,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "ezWtvReswwZaEBThCnW23qtH5uANic2akGY7yh7vZR9".to_string(),
                destination: "6qxghyVLU7sVYhQn6JKziDqb2VMPuDS6Q6rGngnkXdxx".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "4UKfPxrJGEXggv637xCbzethVUGtkv6vay5zCjDSg1Yb".to_string(),
            }
        );

        // fee
        assert_eq!(
            transfers[2],
            TokenTransferDetails {
                amount: 3624,
                ui_amount: 0.000003624,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "ezWtvReswwZaEBThCnW23qtH5uANic2akGY7yh7vZR9".to_string(),
                destination: "Bvtgim23rfocUzxVX9j9QFxTbBnH8JZxnaGLCEkXvjKS".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "4UKfPxrJGEXggv637xCbzethVUGtkv6vay5zCjDSg1Yb".to_string(),
            }
        );

        // the old pump swap amm
        let accounts = Sell::arrange_accounts(&instruction.accounts);
        assert!(accounts.is_none());

        let mut processor = PumpAmmInstructionProcessor::new(token_swap_handler.clone());
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
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    /// https://solscan.io/tx/54cT6UzXbKHn8QuzncXUF81zYM5bq3tWPq4iQvSXorKKB1XUBJaZu7B4dx9dLxThCExMEDK4jAAQDcx9W9FFNCzp
    /// Swap 0.501000002 WSOL for 540,059.097867 $68 icebowl On Pump.fun AMM
    #[tokio::test]
    async fn test_buy_processor() {
        let signature = "54cT6UzXbKHn8QuzncXUF81zYM5bq3tWPq4iQvSXorKKB1XUBJaZu7B4dx9dLxThCExMEDK4jAAQDcx9W9FFNCzp";
        let outer_index = 5;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_amm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 3);

        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                amount: 540059097867,
                ui_amount: 540059.097867,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "B67fRazWFUd4DPgFgLjKXX9dC8vS1UMbXmzYo7YDAqeA".to_string(),
                destination: "9qr6mtX3fELoWGQJyVzHgxuQZptZhmHRMdgZNyGDZkjB".to_string(),
                mint: "2Y6GkQJR93PNL1iYwGcjggoaBRaeTM1p9pC7oCzTpump".to_string(),
                source: "GkcKiF8ku7e54A8NK4UPHW6rmoGfhMeiMHGPpn4yUTkG".to_string(),
            }
        );

        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                authority: "4sDjn4xpDBzd2QiKKGqmprCxeSLaDygC5oijyLLo6qUX".to_string(),
                destination: "39NaF7ehkzNcxXLq9WZdtQ18RFu1rVxs3oQR1a2safoT".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "GHjM41KiTeTiRR2m42RQF4jSpho4C4KKSx4D1ZX7D3Qb".to_string(),
                amount: 501000002,
                decimals: 9,
                ui_amount: 0.501000002,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            }
        );

        // fee
        assert_eq!(
            transfers[2],
            TokenTransferDetails {
                amount: 250001,
                ui_amount: 0.000250001,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "4sDjn4xpDBzd2QiKKGqmprCxeSLaDygC5oijyLLo6qUX".to_string(),
                destination: "94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "GHjM41KiTeTiRR2m42RQF4jSpho4C4KKSx4D1ZX7D3Qb".to_string(),
            }
        );

        // the old pump swap amm
        let user_adas = HashSet::from([
            "9qr6mtX3fELoWGQJyVzHgxuQZptZhmHRMdgZNyGDZkjB".to_string(),
            "GHjM41KiTeTiRR2m42RQF4jSpho4C4KKSx4D1ZX7D3Qb".to_string(),
        ]);
        let vault_adas = HashSet::from([
            "GkcKiF8ku7e54A8NK4UPHW6rmoGfhMeiMHGPpn4yUTkG".to_string(),
            "39NaF7ehkzNcxXLq9WZdtQ18RFu1rVxs3oQR1a2safoT".to_string(),
        ]);
        let fee_adas: HashSet<String> = HashSet::from([
            "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV".to_string(),
            "94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb".to_string(),
        ]);
        let token_swap_accounts = TokenSwapAccounts {
            pair: "".to_string(),
            user_adas,
            vault_adas,
            fee_adas: Some(fee_adas),
            quote_mints: get_pump_amm_quote_mints(),
        };
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = PumpAmmInstructionProcessor::new(token_swap_handler.clone());
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
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    /// https://solscan.io/tx/4SikTiGq3nYZrFd3D5ZwckWVbuCtV2TRgHN6E24t4XpnnemmSk98TZ9SRYCzjK4FSiGEsSr85ep45ARP7i3pkJa7
    /// Swap 0.501000002 WSOL for 540,059.097867 $68 icebowl On Pump.fun AMM
    #[tokio::test]
    async fn test_sell_cornhub_processor() {
        let signature = "4SikTiGq3nYZrFd3D5ZwckWVbuCtV2TRgHN6E24t4XpnnemmSk98TZ9SRYCzjK4FSiGEsSr85ep45ARP7i3pkJa7";
        let outer_index = 4;
        let inner_index = Some(0);
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_amm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 4);

        assert_eq!(
            transfers[0],
            TokenTransferDetails {
                amount: 391682524746,
                ui_amount: 391682.524746,
                decimals: 6,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "BYZ3pR1qfD97h98U4n5VYHykZFMwmvj5e6xDRD4XNoVH".to_string(),
                destination: "7hdWN9EtqM8DxfKN8XH1c7cgLkP1j3G4ztmks33RXvnC".to_string(),
                mint: "7c5Jv9KSCJbct34CqSmtbHpys6u2CtFK9VaPoneGpump".to_string(),
                source: "5j3m8DrJHK2ep26D8bBLFGrd9iGjRbqhmnfcb6YNnWxZ".to_string(),
            }
        );
        assert_eq!(
            transfers[1],
            TokenTransferDetails {
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "94unknx4tmrD8kDpkoYxbQUwJizfyh8T4vhe7MsQZWK".to_string(),
                destination: "DffiRTYTEZgz64jQGX6RazJB1974jrxFbPTbxPghdKv".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "BiuN4oeYfauMtEtT8ot29hiV5A4W8hE3nC5Ec7d1NiYX".to_string(),
                amount: 14472232,
                decimals: 9,
                ui_amount: 0.014472232,
            }
        );

        assert_eq!(
            transfers[2],
            TokenTransferDetails {
                amount: 7258,
                ui_amount: 0.000007258,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "94unknx4tmrD8kDpkoYxbQUwJizfyh8T4vhe7MsQZWK".to_string(),
                destination: "94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "BiuN4oeYfauMtEtT8ot29hiV5A4W8hE3nC5Ec7d1NiYX".to_string(),
            }
        );

        assert_eq!(
            transfers[3],
            TokenTransferDetails {
                amount: 7258,
                ui_amount: 0.000007258,
                decimals: 9,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                authority: "94unknx4tmrD8kDpkoYxbQUwJizfyh8T4vhe7MsQZWK".to_string(),
                destination: "Fwt1r8KThvzs7NU2YPdXNCTMnA4eiAN2gotrwvDJ9PMk".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                source: "BiuN4oeYfauMtEtT8ot29hiV5A4W8hE3nC5Ec7d1NiYX".to_string(),
            }
        );

        let accounts = Sell::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = PumpAmmInstructionProcessor::new(token_swap_handler.clone());
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
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    /// https://solscan.io/tx/4tZNsPeFvmEG5EYGNM5VL4MWJ5gAcAxBJwgymA66wfFFnoQv5huriCV4xveUSunoMdpLzVstGLpCQPG8iDBdAvmx
    /// Swap 63,639.337978 $TRUMPAT for 0.37877569 WSOL On Pump.fun AMM
    #[tokio::test]
    async fn test_trumpat_processor() {
        let signature = "4tZNsPeFvmEG5EYGNM5VL4MWJ5gAcAxBJwgymA66wfFFnoQv5huriCV4xveUSunoMdpLzVstGLpCQPG8iDBdAvmx";
        let outer_index = 5;
        let inner_index = None;
        let (nested_instruction, instruction, _, transaction_metadata) =
            test_with_amm_decoder(signature, outer_index, inner_index).await;
        let instruction = instruction.expect("Instruction is not some");
        let token_swap_handler = get_token_swap_handler().await;

        let inner_instructions = nested_instruction.inner_instructions.clone();
        let transfers = get_inner_token_transfers(&transaction_metadata, &inner_instructions);
        assert_eq!(transfers.len(), 4);

        let accounts = Sell::arrange_accounts(&instruction.accounts);
        let accounts = accounts.expect("Accounts are not some");
        let token_swap_accounts = TokenSwapAccounts::from(accounts);
        let transfers = filter_swap_transfers(&transfers, &token_swap_accounts);
        assert_eq!(transfers.len(), 2);

        let mut processor = PumpAmmInstructionProcessor::new(token_swap_handler.clone());
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
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
