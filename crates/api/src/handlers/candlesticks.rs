use crate::{errors::SonarError, state::AppState};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use serde_with::skip_serializing_none;
use sonar_db::{Candlestick, CandlestickInterval};
use tracing::instrument;

#[skip_serializing_none]
#[derive(Debug, Deserialize)]
pub struct TokenOhlcvQuery {
    pub token: String,
    pub pair: Option<String>,
    pub interval: CandlestickInterval,
    pub limit: Option<usize>,
    pub time_from: Option<i32>,
    pub time_to: Option<i32>,
}

#[instrument(skip(state))]
pub async fn get_candlesticks_by_token(
    State(state): State<AppState>,
    query: Query<TokenOhlcvQuery>,
) -> Result<Json<Value>, SonarError> {
    let pairs = match query.pair.as_deref() {
        Some(pair) => pair.split(',').map(|p| p.trim().to_string()).collect(),
        None => vec![],
    };
    let candlesticks = state
        .db
        .get_candlesticks_by_token(
            &query.token,
            &pairs,
            query.interval.clone(),
            query.limit,
            query.time_from,
            query.time_to,
        )
        .await?;
    Ok(Json(json!(candlesticks)))
}

#[skip_serializing_none]
#[derive(Debug, Deserialize)]
pub struct CandlestickPairQuery {
    pub pair: String,
    pub token: Option<String>,
    pub interval: CandlestickInterval,
    pub limit: Option<usize>,
    pub time_from: Option<i32>,
    pub time_to: Option<i32>,
}

#[instrument(skip(state))]
pub async fn get_candlesticks_by_pair(
    State(state): State<AppState>,
    query: Query<CandlestickPairQuery>,
) -> Result<Json<Vec<Candlestick>>, SonarError> {
    let candlesticks = state
        .db
        .get_candlesticks_by_pair(
            query.pair.as_str(),
            query.token.as_deref(),
            &query.interval,
            query.limit,
            query.time_from,
            query.time_to,
        )
        .await?;
    Ok(Json(candlesticks))
}

#[derive(Debug, Deserialize)]
pub struct AggregateCandlesticksBody {
    pub start_time: i64,
    pub end_time: i64,
    pub interval: CandlestickInterval,
}

/// aggregate_candlesticks aggregates swap events into candlesticks table
#[instrument(skip(state))]
pub async fn aggregate_candlesticks(
    State(state): State<AppState>,
    body: Json<AggregateCandlesticksBody>,
) -> Result<Json<Value>, SonarError> {
    state
        .db
        .aggregate_into_candlesticks(body.start_time, body.end_time, body.interval.clone())
        .await?;
    Ok(Json(json!({
        "success": true,
    })))
}
