use carbon_core::account::AccountMetadata;
use carbon_meteora_damm_v2_decoder::accounts::pool::Pool as MeteoraDammV2Pool;
use carbon_meteora_dlmm_decoder::accounts::lb_pair::LbPair as MeteoraDlmmLbPair;
use carbon_meteora_pools_decoder::accounts::pool::Pool as MeteoraPoolsPool;
use carbon_pump_swap_decoder::accounts::pool::Pool as PumpSwapPool;
use carbon_raydium_amm_v4_decoder::accounts::amm_info::AmmInfo as RaydiumAmmV4AmmInfo;
use carbon_raydium_clmm_decoder::accounts::pool_state::PoolState as RaydiumClmmPoolState;
use carbon_raydium_cpmm_decoder::accounts::pool_state::PoolState as RaydiumCpmmPoolState;
use carbon_token_2022_decoder::accounts::token::Token;
use serde::{Deserialize, Serialize};
use spl_token::state::Account as TokenAccount;

#[derive(Debug, Eq, PartialEq, strum_macros::Display)]
pub enum RequestEvent {
    #[strum(to_string = "account_change")]
    AccountChange,
    #[strum(to_string = "token_holder")]
    TokenHolder,
    #[strum(to_string = "lp")]
    Lp,
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

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LpEvent {
    pub lp: String,
    pub base_mint: String,
    pub quote_mint: String,
}

impl LpEvent {
    pub fn from_meteora_damm_v2(meta: &AccountMetadata, pool: &MeteoraDammV2Pool) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: pool.token_a_mint.to_string(),
            quote_mint: pool.token_b_mint.to_string(),
        }
    }

    pub fn from_meteora_dlmm(meta: &AccountMetadata, lb_pair: &MeteoraDlmmLbPair) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: lb_pair.token_x_mint.to_string(),
            quote_mint: lb_pair.token_y_mint.to_string(),
        }
    }

    pub fn from_meteora_pool(meta: &AccountMetadata, pool: &MeteoraPoolsPool) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: pool.token_a_mint.to_string(),
            quote_mint: pool.token_b_mint.to_string(),
        }
    }

    pub fn from_pump_swap(meta: &AccountMetadata, pool: &PumpSwapPool) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: pool.base_mint.to_string(),
            quote_mint: pool.quote_mint.to_string(),
        }
    }

    pub fn from_raydium_amm_v4(meta: &AccountMetadata, amm_info: &RaydiumAmmV4AmmInfo) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: amm_info.coin_mint.to_string(),
            quote_mint: amm_info.pc_mint.to_string(),
        }
    }

    pub fn from_raydium_clmm(meta: &AccountMetadata, pool: &RaydiumClmmPoolState) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: pool.token_mint0.to_string(),
            quote_mint: pool.token_mint1.to_string(),
        }
    }

    pub fn from_raydium_cpmm(meta: &AccountMetadata, pool: &RaydiumCpmmPoolState) -> Self {
        LpEvent {
            lp: meta.pubkey.to_string(),
            base_mint: pool.token0_mint.to_string(),
            quote_mint: pool.token1_mint.to_string(),
        }
    }
}
