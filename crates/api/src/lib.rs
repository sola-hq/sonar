use crate::{
    shutdown::shutdown_signal_with_handler,
    state::AppState,
    ws::{init_adapter, on_connect, IoProxy},
};

use axum::{
    routing::{get, post},
    Router,
};
use axum_otel::{AxumOtelSpanCreator, Level};
use socketioxide::SocketIo;
use socketioxide_redis::RedisAdapter;
use sonar_db::{make_db_from_env, make_kv_store_from_env, make_redis_subscriber_from_env};
use std::{env::var, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{debug, info};

mod errors;
mod handlers;
mod shutdown;
mod state;
mod ws;

/// Initialize the API server
pub async fn init_api() -> std::io::Result<()> {
    let port: u16 = var("PORT")
        .expect("Expected PORT to be set")
        .parse()
        .expect("Expected PORT to be a number");
    let addrs = format!("0.0.0.0:{}", port);

    debug!("Initializing database...");
    let mut db = make_db_from_env().await.expect("Failed to create database");
    db.initialize().await.expect("Failed to initialize database");
    debug!("Initializing kv");
    let kv_store = make_kv_store_from_env().await.expect("Failed to create KvStore client");
    debug!("Initializing redis subscriber");
    let redis_subscriber =
        make_redis_subscriber_from_env().await.expect("Failed to create RedisSubscriber");

    let state: AppState = AppState { db: Arc::new(db), kv_store: Arc::new(kv_store) };

    let adapter = init_adapter().await.expect("Failed to create RedisAdapter");
    let (socket_layer, io) = SocketIo::builder()
        .with_state(state.clone())
        .with_adapter::<RedisAdapter<_>>(adapter)
        .build_layer();

    io.ns("/", on_connect).await.expect("Failed to create socket io");

    let app = Router::new()
        .route("/top-tokens", get(handlers::tokens::get_top_tokens))
        .route("/candlesticks", get(handlers::candlesticks::get_candlesticks_by_token))
        .route("/token-ohlcv", get(handlers::candlesticks::get_candlesticks_by_token))
        .route("/pair-ohlcv", get(handlers::candlesticks::get_candlesticks_by_pair))
        .route("/ohlcv", post(handlers::candlesticks::aggregate_candlesticks))
        .route("/price", get(handlers::price::get_price))
        .route("/prices", post(handlers::price::get_prices))
        .route("/token-stats", get(handlers::tokens::get_tokens_stats))
        .route("/token-daily-stats", get(handlers::tokens::get_tokens_daily_stats))
        .route("/token", get(handlers::tokens::get_token))
        .route("/tokens", get(handlers::tokens::get_tokens))
        .route("/token", post(handlers::tokens::create_token))
        .route("/trades", get(handlers::swap::get_trades))
        .route("/search", get(handlers::tokens::search))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(AxumOtelSpanCreator::new().level(Level::INFO)),
                )
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
        .layer(socket_layer)
        .route("/health", get(handlers::health::get_health))
        .with_state(state);

    let io_proxy = IoProxy::new(Arc::new(redis_subscriber), Arc::new(io), None);
    io_proxy.spawn_handlers().await.expect("Failed to spawn handlers");

    // Create a `TcpListener` using tokio.
    let listener = TcpListener::bind(addrs).await.expect("Failed to bind to address");
    info!("Starting Server on addrs {:?}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal_with_handler(|| async move {
            info!("Received shutdown signal at {:?}", chrono::Utc::now());
        }))
        .await?;
    info!("Server shutdown at {:?}", chrono::Utc::now());
    Ok(())
}
