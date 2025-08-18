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
#[derive(Debug, Deserialize, Validate, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TopTokensQuery {
    pub limit: Option<usize>,
    pub min_volume: Option<f64>,
    pub min_market_cap: Option<f64>,
    pub timeframe: Option<u64>,
    pub pumpfun: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/top-tokens",
    params(TopTokensQuery),
    responses(
        (status = 200, description = "Top tokens retrieved successfully", body = Vec<TopToken>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(skip(state))]
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
#[derive(Debug, Deserialize, Validate, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TokenStatsQuery {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    #[validate(length(min = 1))]
    pub tokens: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/token-stats",
    params(TokenStatsQuery),
    responses(
        (status = 200, description = "Token stats retrieved successfully", body = Vec<TokenStat>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(skip(state))]
pub async fn get_tokens_stats(
    State(state): State<AppState>,
    query: Query<TokenStatsQuery>,
) -> Result<Json<Vec<TokenStat>>, SonarError> {
    let tokens = state.db.get_token_stats(query.tokens.clone()).await?;
    Ok(Json(tokens))
}

#[utoipa::path(
    get,
    path = "/token-daily-stats",
    params(TokenStatsQuery),
    responses(
        (status = 200, description = "Token daily stats retrieved successfully", body = Vec<TokenDailyStat>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(skip(state))]
pub async fn get_tokens_daily_stats(
    State(state): State<AppState>,
    query: Query<TokenStatsQuery>,
) -> Result<Json<Vec<TokenDailyStat>>, SonarError> {
    let tokens = state.db.get_token_daily_stats(query.tokens.clone()).await?;
    Ok(Json(tokens))
}

#[derive(Debug, Deserialize, Validate, utoipa::IntoParams, utoipa::ToSchema)]
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

#[utoipa::path(
    get,
    path = "/token",
    params(TokenMetadataQuery),
    responses(
        (status = 200, description = "Token retrieved successfully", body = Option<Token>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
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
#[derive(Clone, Debug, Deserialize, Validate, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TokensQuery {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    #[validate(length(min = 1))]
    pub tokens: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/tokens",
    params(TokensQuery),
    responses(
        (status = 200, description = "Tokens retrieved successfully", body = Vec<Token>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
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

#[derive(Debug, Deserialize, Validate, utoipa::IntoParams, utoipa::ToSchema)]
pub struct CreateTokenBody {
    #[serde(flatten)]
    #[allow(dead_code)]
    pub token: Token,
}

#[utoipa::path(
    post,
    path = "/token",
    request_body = CreateTokenBody,
    responses(
        (status = 200, description = "Token created successfully", body = Option<Token>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
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

#[derive(Debug, Deserialize, Validate, utoipa::IntoParams, utoipa::ToSchema)]
pub struct SearchQuery {
    #[validate(length(min = 1, message = "Can not be empty"))]
    #[schema(rename = "s")]
    pub s: String,
}

#[utoipa::path(
    get,
    path = "/search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results retrieved successfully", body = Vec<TokenSearch>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(skip(state))]
pub async fn search(
    State(state): State<AppState>,
    query: Query<SearchQuery>,
) -> Result<Json<Vec<TokenSearch>>, SonarError> {
    query.validate()?;
    let tokens = state.db.search_tokens(&query.s).await?;
    Ok(Json(tokens))
}
