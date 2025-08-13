pub mod cache;
pub mod constants;
use sonar_sol_price::SolPriceCache;
use tracing_otel_extra::init_logging;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let name = env!("CARGO_PKG_NAME");
    init_logging(name).expect("Failed to initialize logging");

    println!("Starting Sol Price CLMM Cache");

    let sol_price_cache = SolPriceCache::new(None, None);
    sol_price_cache.start_price_stream().await.expect("Failed to start price stream");
    println!("Sol Price Cache started");

    println!("Price cache is running. Press Ctrl+C to stop.");

    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            println!("Received Ctrl+C, shutting down gracefully...");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }
}
