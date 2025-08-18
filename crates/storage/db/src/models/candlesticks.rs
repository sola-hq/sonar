use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use strum::{AsRefStr, Display, EnumProperty, EnumString, IntoStaticStr};

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    AsRefStr,
    IntoStaticStr,
    Display,
    EnumProperty,
    Serialize,
    EnumString,
    utoipa::ToSchema
)]
#[serde(rename_all = "lowercase")]
pub enum CandlestickInterval {
    #[strum(serialize = "1s", props(seconds = 1), props(interval = 1))]
    #[schema(rename = "1s")]
    OneSecond,
    #[strum(serialize = "5s", props(seconds = 5), props(interval = 1))]
    #[schema(rename = "5s")]
    FiveSeconds,
    #[strum(serialize = "15s", props(seconds = 15), props(interval = 1))]
    #[schema(rename = "15s")]
    FifteenSeconds,
    #[strum(serialize = "30s", props(seconds = 30), props(interval = 1))]
    #[schema(rename = "30s")]
    ThirtySeconds,
    #[strum(serialize = "1m", props(seconds = 60), props(interval = 60))]
    #[schema(rename = "1m")]
    OneMinute,
    #[strum(serialize = "5m", props(seconds = 300), props(interval = 60))]
    #[schema(rename = "5m")]
    FiveMinutes,
    #[strum(serialize = "15m", props(seconds = 900), props(interval = 60))]
    #[schema(rename = "15m")]
    FifteenMinutes,
    #[strum(serialize = "30m", props(seconds = 1800), props(interval = 60))]
    #[schema(rename = "30m")]
    ThirtyMinutes,
    #[strum(serialize = "1h", props(seconds = 3600), props(interval = 3600))]
    #[schema(rename = "1h")]
    OneHour,
    #[strum(serialize = "4h", props(seconds = 14400), props(interval = 3600))]
    #[schema(rename = "4h")]
    FourHours,
    #[strum(serialize = "1d", props(seconds = 86400), props(interval = 86400))]
    #[schema(rename = "1d")]
    OneDay,
}

impl CandlestickInterval {
    /// Returns the interval in seconds
    pub fn get_seconds(&self) -> i64 {
        self.get_int("seconds").expect("Failed to get seconds")
    }

    /// Returns the interval in candlestick
    pub fn get_candlestick_interval(&self) -> i64 {
        self.get_int("interval").expect("Failed to get interval")
    }
}

impl<'de> Deserialize<'de> for CandlestickInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        CandlestickInterval::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct Candlestick {
    #[serde(rename = "t", alias = "timestamp")]
    #[schema(rename = "t")]
    pub timestamp: u64,
    #[serde(rename = "o", alias = "open")]
    #[schema(rename = "o")]
    pub open: f64,
    #[serde(rename = "h", alias = "high")]
    #[schema(rename = "h")]
    pub high: f64,
    #[serde(rename = "l", alias = "low")]
    #[schema(rename = "l")]
    pub low: f64,
    #[serde(rename = "c", alias = "close")]
    #[schema(rename = "c")]
    pub close: f64,
    #[serde(rename = "v", alias = "volume")]
    #[schema(rename = "v")]
    pub volume: f64,
    #[serde(rename = "vc", alias = "turnover")]
    pub turnover: f64,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CandlestickQuery {
    pub mint: String,
    pub interval: CandlestickInterval,
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candlestick_interval() {
        let interval = CandlestickInterval::OneSecond;
        assert_eq!(interval.get_seconds(), 1);
    }

    #[test]
    fn test_candlestick_interval_display() {
        let interval = CandlestickInterval::OneSecond;
        assert_eq!(format!("{}", interval), "1s");
    }
}
