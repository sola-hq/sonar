use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPoolEvent {
    pub dex: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub pool: String,
    pub timestamp: u64,
}
