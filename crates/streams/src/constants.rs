use solana_pubkey::{pubkey, Pubkey};

/// The program id of the token program
pub const TOKEN_PROGRAM_ID: Pubkey = spl_token::id();

/// The program id of the token 2022 program
pub const TOKEN_2022_PROGRAM_ID: Pubkey = carbon_token_2022_decoder::PROGRAM_ID;

/// The program id of the system program
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");

/// The program id of the raydium amm v4 program
pub const RAYDIUM_AMM_V4_PROGRAM_ID: Pubkey = carbon_raydium_amm_v4_decoder::PROGRAM_ID;

/// The program id of the raydium clmm program
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey = carbon_raydium_clmm_decoder::PROGRAM_ID;

/// The program id of the raydium cpmm program
pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = carbon_raydium_cpmm_decoder::PROGRAM_ID;

/// The program id of the meteora dlmm program
pub const METEORA_DLMM_PROGRAM_ID: Pubkey = carbon_meteora_dlmm_decoder::PROGRAM_ID;

/// The program id of the meteora pools program
pub const METEORA_POOLS_PROGRAM_ID: Pubkey = carbon_meteora_pools_decoder::PROGRAM_ID;

/// The program id of the meteora damm v2 program
pub const METEORA_DAMM_V2_PROGRAM_ID: Pubkey = carbon_meteora_damm_v2_decoder::PROGRAM_ID;

/// The program id of the pump swap program
pub const PUMP_SWAP_PROGRAM_ID: Pubkey = carbon_pump_swap_decoder::PROGRAM_ID;
