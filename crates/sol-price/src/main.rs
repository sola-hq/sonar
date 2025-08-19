pub mod cache;
pub mod constants;
use sonar_sol_price::SolPriceCache;
use std::time::Duration;
use tracing::{error, info};
use tracing_otel_extra::init_logging;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let name = env!("CARGO_PKG_NAME");
    init_logging(name).expect("Failed to initialize logging");

    info!("Starting Sol Price Binance");

    let sol_price_cache = SolPriceCache::new(None, None);
    let price_cache_clone = sol_price_cache.clone();

    // Start price stream in background task
    let price_stream_handle = tokio::spawn(async move {
        if let Err(e) = sol_price_cache.start_price_stream().await {
            error!("Price stream error: {}", e);
            return Err(e);
        }
        Ok(())
    });

    let price_fetch_handle = tokio::spawn(async move {
        let mut last_price = 0.0;
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let price = price_cache_clone.get_price().await;
            info!(
                "Current SOL price: ${:.2} Last price: ${:.2}, Price change: ${:.2}",
                price,
                last_price,
                price - last_price
            );
            last_price = price;
        }
    });

    info!("Price cache is running. Press Ctrl+C to stop.");

    tokio::select! {
        // Handle Ctrl+C signal
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
        result = price_stream_handle => {
            match result {
                Ok(Ok(())) => info!("Price stream completed successfully"),
                Ok(Err(e)) => error!("Price stream error: {}", e),
                Err(e) => error!("Price stream task panicked or was aborted: {}", e),
            }
        }
        result = price_fetch_handle => {
            if let Err(e) = result {
                error!("Price fetch task panicked or was aborted: {}", e);
            }
        }
    }
    info!("Sol Price Cache shutdown complete");
}
