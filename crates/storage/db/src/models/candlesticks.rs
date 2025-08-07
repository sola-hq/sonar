use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
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
    EnumString
)]
#[serde(rename_all = "lowercase")]
pub enum CandlestickInterval {
    #[strum(serialize = "1s", props(seconds = 1), props(interval = 1))]
    OneSecond,
    #[strum(serialize = "5s", props(seconds = 5), props(interval = 1))]
    FiveSeconds,
    #[strum(serialize = "15s", props(seconds = 15), props(interval = 1))]
    FifteenSeconds,
    #[strum(serialize = "30s", props(seconds = 30), props(interval = 1))]
    ThirtySeconds,
    #[strum(serialize = "1m", props(seconds = 60), props(interval = 60))]
    OneMinute,
    #[strum(serialize = "5m", props(seconds = 300), props(interval = 60))]
    FiveMinutes,
    #[strum(serialize = "15m", props(seconds = 900), props(interval = 60))]
    FifteenMinutes,
    #[strum(serialize = "30m", props(seconds = 1800), props(interval = 60))]
    ThirtyMinutes,
    #[strum(serialize = "1h", props(seconds = 3600), props(interval = 3600))]
    OneHour,
    #[strum(serialize = "4h", props(seconds = 14400), props(interval = 3600))]
    FourHours,
    #[strum(serialize = "1d", props(seconds = 86400), props(interval = 86400))]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Candlestick {
    #[serde(rename = "t", alias = "timestamp")]
    pub timestamp: u64,
    #[serde(rename = "o", alias = "open")]
    pub open: f64,
    #[serde(rename = "h", alias = "high")]
    pub high: f64,
    #[serde(rename = "l", alias = "low")]
    pub low: f64,
    #[serde(rename = "c", alias = "close")]
    pub close: f64,
    #[serde(rename = "v", alias = "volume")]
    pub volume: f64,
    #[serde(rename = "vc", alias = "turnover")]
    pub turnover: f64,
}

#[derive(Deserialize)]
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
