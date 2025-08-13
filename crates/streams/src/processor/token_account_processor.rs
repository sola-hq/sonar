use crate::ws::IoProxy;
use carbon_core::{
    account::AccountProcessorInputType, error::CarbonResult, metrics::MetricsCollection,
    processor::Processor,
};
use carbon_token_program_decoder::accounts::TokenProgramAccount;
use serde::{Deserialize, Serialize};
use socketioxide::adapter::Adapter;
use solana_program::program_option::COption;
use solana_pubkey::Pubkey;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
#[repr(u8)]
enum TokenAccountState {
    Uninitialized,
    Initialized,
    Frozen,
}

impl From<spl_token::state::AccountState> for TokenAccountState {
    fn from(state: spl_token::state::AccountState) -> Self {
        match state {
            spl_token::state::AccountState::Uninitialized => Self::Uninitialized,
            spl_token::state::AccountState::Initialized => Self::Initialized,
            spl_token::state::AccountState::Frozen => Self::Frozen,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenAccount {
    pub mint: Pubkey,
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub amount: u64,
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    pub delegate: Option<Pubkey>,
    /// The account's state
    pub state: TokenAccountState,
    /// If is_native.is_some, this is a native token, and the value logs the
    /// rent-exempt reserve. An Account is required to be rent-exempt, so
    /// the value is used by the Processor to ensure that wrapped SOL
    /// accounts do not drop below this threshold.
    pub is_native: Option<u64>,
    /// The amount delegated
    pub delegated_amount: u64,
    /// Optional authority to close the account.
    pub close_authority: Option<Pubkey>,
}

impl From<spl_token::state::Account> for TokenAccount {
    fn from(account: spl_token::state::Account) -> Self {
        Self {
            mint: account.mint,
            owner: account.owner,
            amount: account.amount,
            delegate: match account.delegate {
                COption::Some(delegate) => Some(delegate),
                COption::None => None,
            },
            state: account.state.into(),
            is_native: match account.is_native {
                COption::Some(is_native) => Some(is_native),
                COption::None => None,
            },
            delegated_amount: account.delegated_amount,
            close_authority: match account.close_authority {
                COption::Some(close_authority) => Some(close_authority),
                COption::None => None,
            },
        }
    }
}

pub struct TokenAccountProcessor<A: Adapter> {
    io: Arc<IoProxy<A>>,
}

impl<A: Adapter> TokenAccountProcessor<A> {
    pub fn new(io: Arc<IoProxy<A>>) -> Self {
        Self { io }
    }
}

#[async_trait::async_trait]
impl<A: Adapter> Processor for TokenAccountProcessor<A> {
    type InputType = AccountProcessorInputType<TokenProgramAccount>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (meta, decoded, _solana_account) = data;

        if let TokenProgramAccount::Account(account) = decoded.data {
            let token_account = TokenAccount::from(account);
            if let Ok(value) = serde_json::to_value(token_account) {
                let io = self.io.clone();
                tokio::spawn(async move {
                    if let Err(e) = io.broadcast_account_change(&account.owner, meta, value).await {
                        tracing::warn!("Failed to broadcast token account: {}", e);
                    }
                });
            }
        }
        Ok(())
    }
}
