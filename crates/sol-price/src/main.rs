pub mod cache;
pub mod constants;
use sonar_sol_price::{SolPriceCache, SolPriceCacheTrait};
use tracing_otel_extra::init_logging;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let name = env!("CARGO_PKG_NAME");
    init_logging(name).expect("Failed to initialize logging");

    println!("Starting Sol Price Cpmm Cache");

    let sol_price_cache = SolPriceCache::new(None, None);
    sol_price_cache.start_price_stream().await.expect("Failed to start price stream");
    println!("Sol Price Cache started");

    // 保持程序运行，直到收到 Ctrl+C 信号
    println!("Price cache is running. Press Ctrl+C to stop.");

    // 使用 tokio 的信号处理
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            println!("Received Ctrl+C, shutting down gracefully...");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }
}
