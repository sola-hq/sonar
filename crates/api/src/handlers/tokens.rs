use crate::{
    errors::{SonarError, SonarErrorKind},
    state::AppState,
};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::Json,
};
use futures::future;
use serde::Deserialize;
use serde_with::{formats::CommaSeparator, serde_as, skip_serializing_none, StringWithSeparator};
use sonar_db::{
    models::tokens::{Token, TokenDailyStat, TokenSearch, TokenStat},
    TopToken,
};
use sonar_token_metadata::get_token_metadata_with_data;
use tracing::{instrument, warn};
use validator::Validate;

#[skip_serializing_none]
#[derive(Debug, Deserialize, Validate)]
pub struct TopTokensQuery {
    pub limit: Option<usize>,
    pub min_volume: Option<f64>,
    pub min_market_cap: Option<f64>,
    pub timeframe: Option<u64>,
    pub pumpfun: Option<bool>,
}

pub async fn get_top_tokens(
    State(state): State<AppState>,
    query: Query<TopTokensQuery>,
) -> Result<Json<Vec<TopToken>>, SonarError> {
    let time_range = query.timeframe.unwrap_or(86400); // 24h in seconds
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| SonarErrorKind::InvalidQuery("Failed to get current time".to_string()))?;
    let current_time = current_time.as_secs();
    let start_time = current_time - time_range;
    let limit = query.limit.unwrap_or(10);

    let tokens = state
        .db
        .get_top_tokens(limit, start_time, query.min_volume, query.min_market_cap, query.pumpfun)
        .await?;
    Ok(Json(tokens))
}

#[serde_as]
#[derive(Debug, Deserialize, Validate)]
pub struct TokenStatsQuery {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    #[validate(length(min = 1))]
    pub tokens: Vec<String>,
}

pub async fn get_tokens_stats(
    State(state): State<AppState>,
    query: Query<TokenStatsQuery>,
) -> Result<Json<Vec<TokenStat>>, SonarError> {
    let tokens = state.db.get_token_stats(query.tokens.clone()).await?;
    Ok(Json(tokens))
}

pub async fn get_tokens_daily_stats(
    State(state): State<AppState>,
    query: Query<TokenStatsQuery>,
) -> Result<Json<Vec<TokenDailyStat>>, SonarError> {
    let tokens = state.db.get_token_daily_stats(query.tokens.clone()).await?;
    Ok(Json(tokens))
}

#[derive(Debug, Deserialize, Validate)]
pub struct TokenMetadataQuery {
    #[validate(length(min = 10))]
    pub token: String,
}

// do not return an error here
#[instrument(skip(state))]
pub(crate) async fn get_token_from_state(state: &AppState, mint: &str) -> Option<Token> {
    let token = match get_token_metadata_with_data(mint, &state.kv_store, &state.db).await {
        Ok(token) => token,
        Err(e) => {
            warn!("Failed to get token metadata: {}", e);
            return None;
        }
    };
    Some(token)
}

#[instrument(skip(state))]
pub async fn get_token(
    State(state): State<AppState>,
    query: Query<TokenMetadataQuery>,
) -> Result<Json<Option<Token>>, SonarError> {
    query.validate()?;
    let token = get_token_from_state(&state, &query.token).await;
    Ok(Json(token))
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct TokensQuery {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    #[validate(length(min = 1))]
    pub tokens: Vec<String>,
}

#[instrument(skip(state))]
pub async fn get_tokens(
    State(state): State<AppState>,
    query: Query<TokensQuery>,
) -> Result<Json<Vec<Token>>, SonarError> {
    query.validate()?;
    let mints = query.tokens.clone();
    let tasks = mints.iter().map(|mint| get_token_from_state(&state, mint));
    let tokens = future::join_all(tasks).await;
    let tokens = tokens.into_iter().flatten().collect();
    Ok(Json(tokens))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTokenBody {
    #[serde(flatten)]
    #[allow(dead_code)]
    pub token: Token,
}

#[instrument(skip(state))]
pub async fn create_token(
    State(state): State<AppState>,
    body: Json<CreateTokenBody>,
) -> Result<Json<Option<Token>>, SonarError> {
    body.validate()?;
    let token = body.token.clone();
    state.db.insert_token(&token).await?;
    let token = state.db.get_token(&body.token.token).await?;
    Ok(Json(token))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SearchQuery {
    #[validate(length(min = 1, message = "Can not be empty"))]
    pub s: String,
}

#[instrument(skip(state))]
pub async fn search(
    State(state): State<AppState>,
    query: Query<SearchQuery>,
) -> Result<Json<Vec<TokenSearch>>, SonarError> {
    query.validate()?;
    let tokens = state.db.search_tokens(&query.s).await?;
    Ok(Json(tokens))
}
