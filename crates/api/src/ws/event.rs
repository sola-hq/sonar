#[derive(Debug, Eq, PartialEq, strum_macros::Display)]
pub enum RequestEvent {
    #[strum(to_string = "tokenTrade")]
    TokenTrade,
}

#[derive(Debug, Eq, PartialEq, strum_macros::Display)]
pub enum ResponseEvent {
    #[strum(to_string = "tradeCreated")]
    TradeCreated,
}
