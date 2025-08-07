use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use sonar_db::{make_db_from_env, make_kv_store_from_env, make_message_queue_from_env};
use sonar_ingestor::prelude::*;
use sonar_sol_price::SolPriceCache;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Sonar node")]
#[command(propagate_version = true)]
pub struct Command {
    #[clap(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
/// `sonar node` subcommands
pub enum Subcommands {
    /// helius enhanced websocket
    HeliusWs,
    /// geyser
    Geyser,
    /// websocket
    Ws,
    /// rpc transaction crawler
    Transaction,
    /// rpc block crawler
    #[cfg(feature = "block")]
    Block,
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self) -> anyhow::Result<()> {
        dotenv().ok();
        let db = make_db_from_env().await?;
        info!("db connected");
        let kv_store = make_kv_store_from_env().await?;
        info!("kv connected");
        let message_queue = make_message_queue_from_env().await?;
        info!("message queue connected");

        let kv_store = Arc::new(kv_store);
        let message_queue = Arc::new(message_queue);
        let db = Arc::new(db);

        let price_cache = SolPriceCache::new(Some(kv_store.clone()), Some(message_queue.clone()));
        let price_cache = Arc::new(price_cache);
        let sol_price = price_cache.get_price().await;
        info!("Solana price {}", sol_price);
        assert!(sol_price > 0.0, "Solana price should initialize");

        let mut pipeline = match self.command {
            Subcommands::HeliusWs => {
                info!("Starting helius atlas pipeline...");
                let datasource = make_helius_ws_datasource();
                build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
            }
            Subcommands::Geyser => {
                info!("Starting geyser pipeline...");
                let datasource = make_geyser_datasource();
                build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
            }
            #[cfg(feature = "ws")]
            Subcommands::Ws => {
                info!("Starting ws pipeline...");
                let datasource = make_ws_datasource();
                build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
            }
            Subcommands::Transaction => {
                info!("Starting rpc transaction crawler pipeline...");
                let datasource = make_transaction_crawler_datasource();
                build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
            }
            #[cfg(feature = "block")]
            Subcommands::Block => {
                info!("Starting rpc block crawler pipeline...");
                let datasource = make_block_crawler_datasource();
                build_pipeline(datasource, db, kv_store.clone(), message_queue.clone())?
            }
        };
        tokio::spawn(async move {
            if let Err(e) = price_cache.start_price_stream().await {
                error!("Error in SOL price stream: {}", e);
            }
        });

        pipeline.run().await?;
        Ok(())
    }
}
