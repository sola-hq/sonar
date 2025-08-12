use carbon_core::account::AccountMetadata;
use carbon_token_2022_decoder::accounts::token::Token;
use serde::{Deserialize, Serialize};
use spl_token::state::Account as TokenAccount;

#[derive(Debug, Eq, PartialEq, strum_macros::Display)]
pub enum RequestEvent {
    #[strum(to_string = "token_holder")]
    TokenHolder,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u8")]
pub enum TokenProgram {
    Token = 0,
    Token2022 = 1,
    System = 2,
}

impl From<TokenProgram> for u8 {
    fn from(program: TokenProgram) -> u8 {
        match program {
            TokenProgram::Token => 0,
            TokenProgram::Token2022 => 1,
            TokenProgram::System => 2,
        }
    }
}

#[derive(Debug, Eq, PartialEq, strum_macros::Display)]
pub enum ResponseEvent {
    TokenHolderEvent(TokenHolderEvent),
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenHolderEvent {
    pub address: String,
    pub mint: String,
    pub amount: u64,
    pub program: TokenProgram,
}

impl TokenHolderEvent {
    pub fn from_system_account(meta: AccountMetadata, account: solana_account::Account) -> Self {
        TokenHolderEvent {
            address: meta.pubkey.to_string(),
            mint: '0'.to_string(),
            amount: account.lamports,
            program: TokenProgram::System,
        }
    }

    pub fn from_token_account(meta: AccountMetadata, account: TokenAccount) -> Self {
        TokenHolderEvent {
            address: meta.pubkey.to_string(),
            mint: account.mint.to_string(),
            amount: account.amount,
            program: TokenProgram::Token,
        }
    }

    pub fn from_token_2022_account(meta: AccountMetadata, account: Token) -> Self {
        TokenHolderEvent {
            address: meta.pubkey.to_string(),
            mint: account.mint.to_string(),
            amount: account.amount,
            program: TokenProgram::Token2022,
        }
    }
}
