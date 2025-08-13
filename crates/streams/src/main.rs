use anyhow::{Context, Result};
use sonar_streams::{app::App, datasource::make_ws_datasource};
use std::env;
use tracing::info;
use tracing_otel_extra::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let name = env!("CARGO_PKG_NAME");
    init_logging(name).expect("Failed to initialize logging");

    info!("Starting Streams service...");

    let datasources = make_ws_datasource();
    let app = App::new();
    app.run(datasources).await.context("Failed to run app")?;

    info!("Pipeline completed successfully");
    Ok(())
}
