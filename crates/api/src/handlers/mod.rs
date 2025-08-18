use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod candlesticks;
pub mod health;
pub mod price;
pub mod swap;
pub mod tokens;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::get_health,
        price::get_price,
				price::get_prices,
				candlesticks::aggregate_candlesticks,
				candlesticks::get_candlesticks_by_token,
				candlesticks::get_candlesticks_by_pair,
				swap::get_trades,
				tokens::create_token,
				tokens::get_token,
				tokens::get_tokens,
				tokens::get_tokens_daily_stats,
				tokens::get_tokens_stats,
				tokens::search,
				tokens::get_top_tokens,
    ),
    components(
        schemas(
            health::HealthResponse,
            sonar_db::models::tokens::TokenPrice,
            price::PriceQuery,
            price::PricesQuery,
						candlesticks::AggregateCandlesticksBody,
            candlesticks::TokenOhlcvQuery,
            candlesticks::CandlestickPairQuery,
            tokens::TopTokensQuery,
            tokens::TokenStatsQuery,
            tokens::TokenMetadataQuery,
            tokens::TokensQuery,
            tokens::CreateTokenBody,
            tokens::SearchQuery,
        )
    ),
    tags(
        (name = "sonar-api", description = "Sonar API")
    )
)]
pub struct ApiDoc;

/// Get the API documentation
pub fn api_doc() -> SwaggerUi {
    SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi())
}
