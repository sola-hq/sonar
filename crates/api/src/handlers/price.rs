use crate::{errors::SonarError, state::AppState};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_with::skip_serializing_none;
use sonar_db::models::tokens::TokenPrice;
use tracing::instrument;
use validator::Validate;

#[skip_serializing_none]
#[derive(Debug, Deserialize, Validate)]
pub struct PriceQuery {
    #[validate(length(min = 10))]
    pub token: String,
    #[validate(range(min = 0))]
    pub timestamp: Option<i32>,
}

#[instrument(skip(state))]
pub async fn get_price(
    State(state): State<AppState>,
    query: Query<PriceQuery>,
) -> Result<Json<TokenPrice>, SonarError> {
    query.validate()?;
    let now = Utc::now().timestamp() as i32;

    // If no timestamp specified, try to get latests price
    if query.timestamp.is_none() {
        if let Some(price) = state.kv_store.get_price(&query.token).await? {
            return Ok(Json(TokenPrice {
                token: query.token.clone(),
                timestamp: now,
                price: Some(price.price),
                neatest_timestamp: Some(price.timestamp as i32),
            }));
        }
    }

    // Get price for specific timestamp or fallback to latest
    let timestamp = query.timestamp.unwrap_or(now);
    let price = state.db.get_price(&query.token, timestamp).await?;

    Ok(Json(price))
}

#[derive(Debug, Deserialize, Validate)]
pub struct PricesQuery {
    #[validate(length(min = 10))]
    pub token: String,
    #[validate(range(min = 0))]
    pub timestamp: i32,
}

#[instrument(skip(state))]
pub async fn get_prices(
    State(state): State<AppState>,
    query: Json<Vec<PricesQuery>>,
) -> Result<Json<Vec<TokenPrice>>, SonarError> {
    query.validate()?;

    let queries = query.iter().map(|q| (q.token.as_str(), q.timestamp)).collect();
    let prices = state.db.get_prices(queries).await?;

    Ok(Json(prices))
}
