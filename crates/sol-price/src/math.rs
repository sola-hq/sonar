// https://github.com/raydium-io/raydium-sdk-V2/blob/master/src/raydium/clmm/utils/math.ts

use bigdecimal::BigDecimal;
use num_traits::{One, Pow};

pub struct SqrtPriceMath;

impl SqrtPriceMath {
    pub fn sqrt_price_x64_to_price(
        sqrt_price_x64: u64,
        decimals_a: u32,
        decimals_b: u32,
    ) -> BigDecimal {
        BigDecimal::from(sqrt_price_x64)
            .pow(2)
            .mul(BigDecimal::from(10).pow(decimals_a - decimals_b))
            .to_f64()
            .unwrap()
    }

    pub fn price_to_sqrt_price_x64(price: Decimal, decimals_a: u32, decimals_b: u32) -> BN {
        MathUtil::decimal_to_x64(price.mul(Decimal::pow(10, decimals_b - decimals_a)).sqrt())
    }
}
