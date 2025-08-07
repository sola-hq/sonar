use crate::constants::{TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID};
use carbon_core::{
    deserialize::ArrangeAccounts,
    instruction::{DecodedInstruction, InstructionDecoder, NestedInstruction},
    transaction::TransactionMetadata,
};
use carbon_token_2022_decoder::{
    instructions::{
        transfer::{Transfer as Token2022Transfer, TransferInstructionAccounts},
        transfer_checked::{
            TransferChecked as Token2022TransferChecked, TransferCheckedInstructionAccounts,
        },
        Token2022Instruction,
    },
    Token2022Decoder,
};
use carbon_token_program_decoder::{
    instructions::{
        transfer::{Transfer, TransferAccounts},
        transfer_checked::{TransferChecked, TransferCheckedAccounts},
        TokenProgramInstruction,
    },
    TokenProgramDecoder,
};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use spl_token::amount_to_ui_amount;
use std::{collections::HashMap, sync::LazyLock};
use tracing::error;

/// Represents the details of a token transfer instruction
///
/// This struct contains all the relevant information about a token transfer,
/// including the source and destination accounts, token mint, authority,
/// amount and decimal precision.
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct TokenTransferDetails {
    /// The ID of the token program executing the transfer (Token or Token-2022)
    pub program_id: String,
    /// The source account address the tokens are being transferred from
    pub source: String,
    /// The destination account address the tokens are being transferred to
    pub destination: String,
    /// The mint address of the token
    pub mint: String,
    /// The account authorized of source account
    pub authority: String,
    /// The decimal precision of the token
    pub decimals: u8,
    /// The raw token amount being transferred (not adjusted for decimals)
    pub amount: u64,
    /// The token amount in UI format (adjusted for decimals)
    pub ui_amount: f64,
}

/// Implement the From trait for TokenTransferDetails for account types with mint field
macro_rules! impl_into_token_transfer_details_with_mint {
    ($account_type:ty, $program_id:expr) => {
        impl From<$account_type> for TokenTransferDetails {
            fn from(accounts: $account_type) -> Self {
                Self {
                    program_id: $program_id.to_string(),
                    source: accounts.source.to_string(),
                    destination: accounts.destination.to_string(),
                    authority: accounts.authority.to_string(),
                    mint: accounts.mint.to_string(),
                    decimals: 0,
                    amount: 0,
                    ui_amount: 0.0,
                }
            }
        }
    };
}

/// Implement the From trait for TokenTransferDetails for account types without mint field
macro_rules! impl_into_token_transfer_details_without_mint {
    ($account_type:ty, $program_id:expr) => {
        impl From<$account_type> for TokenTransferDetails {
            fn from(accounts: $account_type) -> Self {
                Self {
                    program_id: $program_id.to_string(),
                    source: accounts.source.to_string(),
                    destination: accounts.destination.to_string(),
                    authority: accounts.authority.to_string(),
                    mint: String::new(),
                    decimals: 0,
                    amount: 0,
                    ui_amount: 0.0,
                }
            }
        }
    };
}

// Implement From trait for different account types
impl_into_token_transfer_details_without_mint!(TransferAccounts, TOKEN_PROGRAM_ID);
impl_into_token_transfer_details_with_mint!(TransferCheckedAccounts, TOKEN_PROGRAM_ID);
impl_into_token_transfer_details_without_mint!(TransferInstructionAccounts, TOKEN_2022_PROGRAM_ID);
impl_into_token_transfer_details_with_mint!(
    TransferCheckedInstructionAccounts,
    TOKEN_2022_PROGRAM_ID
);

/// A decoder for Solana SPL token transfer instructions
///
/// This struct provides methods to decode and extract token transfer details from
/// both the standard Token program and the Token-2022 program instructions.
pub struct SPLTokenDecoder {
    token_decoder: TokenProgramDecoder,
    token_2022_decoder: Token2022Decoder,
}

/// A static instance of SPLTokenDecoder for global access
pub static SPL_TOKEN_DECODER: LazyLock<SPLTokenDecoder> = LazyLock::new(SPLTokenDecoder::new);

/// Process a standard Token program instruction to extract transfer details
pub fn process_token_transfer(
    instruction: DecodedInstruction<TokenProgramInstruction>,
) -> Option<TokenTransferDetails> {
    if !instruction.program_id.eq(&TOKEN_PROGRAM_ID) {
        return None;
    }

    match &instruction.data {
        TokenProgramInstruction::Transfer(t) => Transfer::arrange_accounts(&instruction.accounts)
            .map(|accounts| {
                let mut details = TokenTransferDetails::from(accounts);
                details.amount = t.amount;
                details
            }),
        TokenProgramInstruction::TransferChecked(t) => {
            TransferChecked::arrange_accounts(&instruction.accounts).map(|accounts| {
                let mut details = TokenTransferDetails::from(accounts);
                details.amount = t.amount;
                details.decimals = t.decimals;
                details.ui_amount = amount_to_ui_amount(t.amount, t.decimals);
                details
            })
        }
        _ => None,
    }
}

/// Process a Token-2022 program instruction to extract transfer details
pub fn process_token_2022_transfer(
    instruction: DecodedInstruction<Token2022Instruction>,
) -> Option<TokenTransferDetails> {
    if !instruction.program_id.eq(&TOKEN_2022_PROGRAM_ID) {
        return None;
    }

    match &instruction.data {
        Token2022Instruction::Transfer(t) => {
            Token2022Transfer::arrange_accounts(&instruction.accounts).map(|accounts| {
                let mut details = TokenTransferDetails::from(accounts);
                details.amount = t.amount;
                details
            })
        }
        Token2022Instruction::TransferChecked(t) => {
            Token2022TransferChecked::arrange_accounts(&instruction.accounts).map(|accounts| {
                let mut details = TokenTransferDetails::from(accounts);
                details.amount = t.amount;
                details.decimals = t.decimals;
                details.ui_amount = amount_to_ui_amount(t.amount, t.decimals);
                details
            })
        }
        _ => None,
    }
}

impl SPLTokenDecoder {
    /// Create a new SPL token decoder
    pub fn new() -> Self {
        Self { token_decoder: TokenProgramDecoder, token_2022_decoder: Token2022Decoder }
    }

    /// Try to decode a standard Token program transfer instruction
    pub fn try_decode_token_transfer(
        &self,
        instruction: &solana_instruction::Instruction,
    ) -> Option<TokenTransferDetails> {
        if instruction.program_id != TOKEN_PROGRAM_ID {
            return None;
        }
        self.token_decoder.decode_instruction(instruction).and_then(process_token_transfer)
    }

    /// Try to decode a Token-2022 program transfer instruction
    pub fn try_decode_token_2022_transfer(
        &self,
        instruction: &solana_instruction::Instruction,
    ) -> Option<TokenTransferDetails> {
        if instruction.program_id != TOKEN_2022_PROGRAM_ID {
            return None;
        }

        self.token_2022_decoder
            .decode_instruction(instruction)
            .and_then(process_token_2022_transfer)
    }

    /// Decode a token transfer instruction and enrich it with vault information
    pub fn decode_token_transfer_with_vaults(
        &self,
        mint_details: &HashMap<String, MintDetail>,
        instruction: &solana_instruction::Instruction,
    ) -> Option<TokenTransferDetails> {
        let details = match instruction.program_id {
            TOKEN_PROGRAM_ID => self.try_decode_token_transfer(instruction),
            TOKEN_2022_PROGRAM_ID => self.try_decode_token_2022_transfer(instruction),
            _ => None,
        };
        details.map(|mut details| {
            update_token_transfer_details(&mut details, mint_details);
            details
        })
    }

    /// Decode token transfers from a list of nested instructions
    ///
    /// This method processes a list of nested instructions, extracting and enriching
    /// token transfer details from any transfer instructions found.
    pub fn decode_token_transfers_from_instructions(
        &self,
        nested_instructions: &[NestedInstruction],
        mint_details: &HashMap<String, MintDetail>,
    ) -> Vec<TokenTransferDetails> {
        nested_instructions
            .iter()
            .filter_map(|instruction| {
                self.decode_token_transfer_with_vaults(mint_details, &instruction.instruction)
            })
            .collect()
    }
}

impl Default for SPLTokenDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents details about a token mint
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MintDetail {
    pub mint: String,
    pub owner: String,
    pub decimals: u8,
}

impl From<&solana_transaction_status::TransactionTokenBalance> for MintDetail {
    fn from(balance: &solana_transaction_status::TransactionTokenBalance) -> Self {
        Self {
            mint: balance.mint.clone(),
            owner: balance.owner.clone(),
            decimals: balance.ui_token_amount.decimals,
        }
    }
}

/// Update mint details from transaction token balances
pub fn update_token_accounts_from_meta<'a>(
    signature: &Signature,
    accounts: &[Pubkey],
    balances: &[solana_transaction_status::TransactionTokenBalance],
    mint_details: &'a mut HashMap<String, MintDetail>,
) -> &'a mut HashMap<String, MintDetail> {
    for balance in balances {
        if let Some(pubkey) = accounts.get(balance.account_index as usize) {
            mint_details.insert(pubkey.to_string(), MintDetail::from(balance));
        } else {
            error!("Invalid account_index {} for signature: {}", balance.account_index, signature);
        }
    }
    mint_details
}

/// Update token transfer details with vault and mint information
pub fn update_token_transfer_details(
    details: &mut TokenTransferDetails,
    mint_details: &HashMap<String, MintDetail>,
) {
    if let Some(mint_detail) = mint_details.get(&details.source) {
        update_details_from_mint(details, mint_detail);
    } else if let Some(mint_detail) = mint_details.get(&details.destination) {
        update_details_from_mint(details, mint_detail);
    }
}

/// Helper function to update token details from mint information
fn update_details_from_mint(
    token_transfer_details: &mut TokenTransferDetails,
    mint_detail: &MintDetail,
) {
    token_transfer_details.mint = mint_detail.mint.clone();
    token_transfer_details.decimals = mint_detail.decimals;
    token_transfer_details.ui_amount =
        amount_to_ui_amount(token_transfer_details.amount, mint_detail.decimals);
}

/// Extract mint details from transaction metadata
pub fn extra_mint_details_from_tx_metadata(
    transaction_metadata: &TransactionMetadata,
) -> HashMap<String, MintDetail> {
    let mut mint_details = HashMap::new();
    let account_keys = transaction_metadata.message.static_account_keys().to_vec();
    let loaded_addresses = transaction_metadata.meta.loaded_addresses.clone();
    let accounts_address =
        [account_keys, loaded_addresses.writable, loaded_addresses.readonly].concat();

    let meta = &transaction_metadata.meta;
    if let Some(pre_balances) = meta.pre_token_balances.as_ref() {
        update_token_accounts_from_meta(
            &transaction_metadata.signature,
            &accounts_address,
            pre_balances,
            &mut mint_details,
        );
    }
    if let Some(post_balances) = meta.post_token_balances.as_ref() {
        update_token_accounts_from_meta(
            &transaction_metadata.signature,
            &accounts_address,
            post_balances,
            &mut mint_details,
        );
    }
    mint_details
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_swaps::get_nested_instruction;
    use carbon_core::instruction::InstructionDecoder;
    use carbon_meteora_dlmm_decoder::MeteoraDlmmDecoder;
    use carbon_raydium_amm_v4_decoder::RaydiumAmmV4Decoder;
    use dotenvy::dotenv;

    /// https://solscan.io/tx/3m4LERWUekW7im8rgu8QgpSJA8a9yEYL3gDvorbd5YpkXarrL3PGoVmyFyQzd1Pw9oZiQy2LPUjaG8Xr4p433kwn
    /// 3.2 - Raydium Liquidity Pool V4: raydium:swap
    #[tokio::test]
    async fn test_amm_v4_swap() {
        dotenv().ok();
        let signature = "3m4LERWUekW7im8rgu8QgpSJA8a9yEYL3gDvorbd5YpkXarrL3PGoVmyFyQzd1Pw9oZiQy2LPUjaG8Xr4p433kwn";
        let outer_index = 2;
        let inner_index = Some(1);
        let decoder = RaydiumAmmV4Decoder;
        let token_decoder = TokenProgramDecoder;
        let (nested_instruction, _, transaction_metadata) =
            get_nested_instruction(signature, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoded_instruction = decoder.decode_instruction(&nested_instruction.instruction);
        let _decoded_instruction = decoded_instruction.expect("Failed to decode instruction");
        assert_eq!(2, nested_instruction.inner_instructions.len());

        let mint_details = extra_mint_details_from_tx_metadata(&transaction_metadata);
        let instruction = nested_instruction.inner_instructions[0].instruction.clone();
        let decoded_instruction = token_decoder.decode_instruction(&instruction);
        if let Some(decoded_instruction) = decoded_instruction {
            let details = process_token_transfer(decoded_instruction);
            let details = details.expect("Failed to get token transfer details");
            assert!(details.source == "89YMNsMDmHeMhT3BiDTcryRuxWSn24B31Gf5H9N2Z8Zu");
            assert!(details.destination == "7bxbfwXi1CY7zWUXW35PBMZjhPD27SarVuHaehMzR2Fn");
            assert!(details.authority == "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB");
            assert!(details.amount == 6000000000);
            assert!(details.mint.is_empty());

            // expand the mint details
            let details =
                SPL_TOKEN_DECODER.decode_token_transfer_with_vaults(&mint_details, &instruction);
            let details = details.expect("Failed to get token transfer details");
            assert!(details.source == "89YMNsMDmHeMhT3BiDTcryRuxWSn24B31Gf5H9N2Z8Zu");
            assert!(details.destination == "7bxbfwXi1CY7zWUXW35PBMZjhPD27SarVuHaehMzR2Fn");
            assert!(details.authority == "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB");
            assert!(details.amount == 6000000000);
            assert!(details.mint == "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump");
            assert!(details.ui_amount == 6000.0);
        }

        let instruction = nested_instruction.inner_instructions[1].instruction.clone();
        let decoded_instruction = token_decoder.decode_instruction(&instruction);
        if let Some(decoded_instruction) = decoded_instruction {
            let details = process_token_transfer(decoded_instruction);
            assert!(details.is_some());
            let details = details.expect("Failed to get token transfer details");
            assert!(details.source == "F6iWqisguZYprVwp916BgGR7d5ahP6Ev5E213k8y3MEb");
            assert!(details.destination == "7x4VcEX8aLd3kFsNWULTp1qFgVtDwyWSxpTGQkoMM6XX");
            assert!(details.authority == "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
            assert!(details.amount == 16337636830);

            // expand the mint details
            let details =
                SPL_TOKEN_DECODER.decode_token_transfer_with_vaults(&mint_details, &instruction);
            let details = details.expect("Failed to get token transfer details");
            assert!(details.source == "F6iWqisguZYprVwp916BgGR7d5ahP6Ev5E213k8y3MEb");
            assert!(details.destination == "7x4VcEX8aLd3kFsNWULTp1qFgVtDwyWSxpTGQkoMM6XX");
            assert!(details.authority == "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
            assert!(details.amount == 16337636830);
            assert!(details.mint == "So11111111111111111111111111111111111111112");
            assert!(details.ui_amount == 16.33763683);
        }
    }

    /// https://solscan.io/tx/3m4LERWUekW7im8rgu8QgpSJA8a9yEYL3gDvorbd5YpkXarrL3PGoVmyFyQzd1Pw9oZiQy2LPUjaG8Xr4p433kwn
    /// 3.6 - Meteora DLMM Program: swap
    #[tokio::test]
    async fn test_meteora_dlmm_swap() {
        dotenv().ok();
        let signature = "3m4LERWUekW7im8rgu8QgpSJA8a9yEYL3gDvorbd5YpkXarrL3PGoVmyFyQzd1Pw9oZiQy2LPUjaG8Xr4p433kwn";
        let outer_index = 2;
        let inner_index = Some(3);
        let decoder = MeteoraDlmmDecoder;
        let token_decoder = TokenProgramDecoder;
        let (nested_instruction, _, transaction_metadata) =
            get_nested_instruction(signature, outer_index, inner_index)
                .await
                .expect("Failed to get nested instruction");
        let decoded_instruction = decoder.decode_instruction(&nested_instruction.instruction);
        let _decoded_instruction = decoded_instruction.expect("Failed to decode instruction");

        assert_eq!(3, nested_instruction.inner_instructions.len());

        let instruction = nested_instruction.inner_instructions[0].instruction.clone();
        let decoded_instruction = token_decoder.decode_instruction(&instruction);
        let mint_details = extra_mint_details_from_tx_metadata(&transaction_metadata);
        if let Some(decoded_instruction) = decoded_instruction {
            let details = process_token_transfer(decoded_instruction);
            assert!(details.is_some());
            let details = details.expect("Failed to get token transfer details");
            assert_eq!(
                details,
                TokenTransferDetails {
                    mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_string(),
                    amount: 24000000000,
                    ui_amount: 24000.0,
                    decimals: 6,
                    program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                    source: "89YMNsMDmHeMhT3BiDTcryRuxWSn24B31Gf5H9N2Z8Zu".to_string(),
                    destination: "CMVrNeYhZnqdbZfQuijgcNvCfvTJN2WKvKSnt2q3HT6N".to_string(),
                    authority: "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB".to_string(),
                }
            );

            // expand the mint details
            let details =
                SPL_TOKEN_DECODER.decode_token_transfer_with_vaults(&mint_details, &instruction);
            let details = details.expect("Failed to get token transfer details");
            assert_eq!(
                details,
                TokenTransferDetails {
                    mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_string(),
                    amount: 24000000000,
                    ui_amount: 24000.0,
                    decimals: 6,
                    program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                    source: "89YMNsMDmHeMhT3BiDTcryRuxWSn24B31Gf5H9N2Z8Zu".to_string(),
                    destination: "CMVrNeYhZnqdbZfQuijgcNvCfvTJN2WKvKSnt2q3HT6N".to_string(),
                    authority: "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB".to_string(),
                }
            );
        }

        let instruction = nested_instruction.inner_instructions[1].instruction.clone();
        let decoded_instruction = token_decoder.decode_instruction(&instruction);
        if let Some(decoded_instruction) = decoded_instruction {
            let details = process_token_transfer(decoded_instruction);
            assert!(details.is_some());
            let details = details.expect("Failed to get token transfer details");

            assert_eq!(
                details,
                TokenTransferDetails {
                    mint: "So11111111111111111111111111111111111111112".to_string(),
                    amount: 65256388526,
                    ui_amount: 65.256388526,
                    decimals: 9,
                    program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                    source: "5EfbkfLpaz9mHeTN6FnhtN8DTdMGZDRURYcsQ1f1Utg6".to_string(),
                    destination: "7x4VcEX8aLd3kFsNWULTp1qFgVtDwyWSxpTGQkoMM6XX".to_string(),
                    authority: "6wJ7W3oHj7ex6MVFp2o26NSof3aey7U8Brs8E371WCXA".to_string(),
                }
            );

            // expand the mint details
            let details =
                SPL_TOKEN_DECODER.decode_token_transfer_with_vaults(&mint_details, &instruction);
            let details: TokenTransferDetails =
                details.expect("Failed to get token transfer details");

            assert_eq!(
                details,
                TokenTransferDetails {
                    mint: "So11111111111111111111111111111111111111112".to_string(),
                    amount: 65256388526,
                    ui_amount: 65.256388526,
                    decimals: 9,
                    program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                    source: "5EfbkfLpaz9mHeTN6FnhtN8DTdMGZDRURYcsQ1f1Utg6".to_string(),
                    destination: "7x4VcEX8aLd3kFsNWULTp1qFgVtDwyWSxpTGQkoMM6XX".to_string(),
                    authority: "6wJ7W3oHj7ex6MVFp2o26NSof3aey7U8Brs8E371WCXA".to_string(),
                }
            );
        }
    }
}
