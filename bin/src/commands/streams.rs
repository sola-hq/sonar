use anyhow::Context;
use clap::{Parser, Subcommand};

use dotenvy::dotenv;
use sonar_streams::{
    app::App,
    datasource::{make_geyser_datasource, make_ws_datasource},
};
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Sonar streams")]
pub struct Command {
    #[clap(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
/// `sonar node` subcommands
pub enum Subcommands {
    /// geyser
    Geyser,
    /// websocket
    Ws,
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self) -> anyhow::Result<()> {
        dotenv().ok();

        let app = App::new();

        match self.command {
            Subcommands::Geyser => {
                info!("Starting geyser pipeline...");
                let datasources = vec![make_geyser_datasource()];
                app.run(datasources).await.context("Failed to run app")?;
            }
            Subcommands::Ws => {
                info!("Starting ws datasource...");
                let datasources = vec![make_ws_datasource()];
                app.run(datasources).await.context("Failed to run app")?;
            }
        };
        Ok(())
    }
}
