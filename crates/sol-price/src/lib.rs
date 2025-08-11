pub mod cache;
#[cfg(not(feature = "binance"))]
pub mod clmm;
pub mod constants;
pub mod math;

#[cfg(feature = "binance")]
pub mod binance;

#[cfg(not(feature = "binance"))]
pub use clmm::SolPriceCache;

#[cfg(feature = "binance")]
pub use binance::SolPriceCache;

pub use cache::{get_sol_price, SolPriceCacheTrait, SOL_PRICE_CACHE};
