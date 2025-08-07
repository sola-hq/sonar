use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use sonar_db::{make_db_from_env, make_kv_store_from_env, make_message_queue_from_env};
use sonar_ingestor::prelude::{
    build_pipeline, make_block_crawler_datasource, make_geyser_datasource,
    make_helius_ws_datasource, make_transaction_crawler_datasource, make_ws_datasource,
};
use sonar_sol_price::SolPriceCache;
use std::sync::Arc;
use tracing::{error, info};
use tracing_otel_extra::init_logging;

#[derive(Parser)]
#[clap(version, about)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

/// Work seamlessly with sonar from the command line.
///
/// See `sonar --help` for more information.
#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(name = "helius-ws", about = "Start node with helius websocket datasource")]
    HeliusWs,
    #[command(name = "geyser", about = "Start node with geyser datasource")]
    Geyser,
    #[command(name = "block", about = "Start node with block crawler datasource")]
    Block,
    #[command(name = "transaction", about = "Start node with transaction crawler datasource")]
    Transaction,
    #[command(name = "ws", about = "Start node with ws datasource")]
    Ws,
}

impl Args {
    pub fn from_env_and_args() -> Self {
        dotenv().ok();
        Self::parse()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let name = env!("CARGO_PKG_NAME");
    let _guard = init_logging(name).expect("Failed to initialize logging");

    let opt = Args::from_env_and_args();
    let db = make_db_from_env().await?;
    info!("db connected");
    let kv_store = make_kv_store_from_env().await?;
    info!("kv connected");
    let message_queue = make_message_queue_from_env().await?;
    info!("message queue connected");

    let db = Arc::new(db);
    let kv_store = Arc::new(kv_store);
    let message_queue = Arc::new(message_queue);

    let mut pipeline = match opt.command {
        Commands::HeliusWs => {
            info!("Starting helius websocket pipeline...");
            let datasource = make_helius_ws_datasource();
            build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
        }
        Commands::Geyser => {
            info!("Starting geyser pipeline...");
            let datasource = make_geyser_datasource();
            build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
        }
        Commands::Block => {
            info!("Starting block pipeline...");
            let datasource = make_block_crawler_datasource();
            build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
        }
        Commands::Transaction => {
            info!("Starting transaction pipeline...");
            let datasource = make_transaction_crawler_datasource();
            build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
        }
        Commands::Ws => {
            info!("Starting ws pipeline...");
            let datasource = make_ws_datasource();
            build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
        }
    };

    let price_cache = SolPriceCache::new(Some(kv_store.clone()), Some(message_queue.clone()));
    let price_cache = Arc::new(price_cache);

    // Initialize the price cache
    info!("Solana price: {}", price_cache.get_price().await);

    // Spawn the price stream in a separate task
    tokio::spawn(async move {
        if let Err(e) = price_cache.start_price_stream().await {
            error!("Error in price stream: {}", e);
        }
    });

    pipeline.run().await?;
    Ok(())
}
