pub mod cache;
pub mod constants;
#[cfg(not(feature = "binance"))]
pub mod cpmm;

#[cfg(feature = "binance")]
pub mod binance;

#[cfg(not(feature = "binance"))]
pub use cpmm::SolPriceCache;

#[cfg(feature = "binance")]
pub use binance::SolPriceCache;

pub use cache::{get_sol_price, SolPriceCacheTrait, SOL_PRICE_CACHE};
