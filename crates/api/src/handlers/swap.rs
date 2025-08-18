use crate::{errors::SonarError, state::AppState};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use sonar_db::Trade;
use tracing::instrument;

#[derive(Deserialize, Debug, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TradeQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/trades",
    params(TradeQuery),
    responses(
        (status = 200, description = "Trades retrieved successfully", body = Vec<Trade>),
        (status = 400, description = "Invalid request parameters"),
        (status = 500, description = "Internal server error")
    )
)]
#[instrument(skip(state))]
pub async fn get_trades(
    State(state): State<AppState>,
    query: Query<TradeQuery>,
) -> Result<Json<Vec<Trade>>, SonarError> {
    let swaps = state
        .db
        .get_trades(
            query.address.as_deref(),
            query.token.as_deref(),
            query.pair.as_deref(),
            query.signature.as_deref(),
            query.limit,
            query.offset,
        )
        .await?;
    Ok(Json(swaps))
}
