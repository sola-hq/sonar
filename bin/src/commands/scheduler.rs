use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use sonar_db::make_db_from_env;
use sonar_scheduler::{
    job::{run_jobs, stop_jobs},
    shutdown_signal_with_handler, JobScheduler,
};
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Sonar scheduler")]
#[command(propagate_version = true)]
pub struct Command {
    #[clap(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
/// `sonar node` subcommands
pub enum Subcommands {
    /// candlestick aggregator
    Candlestick,
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self) -> anyhow::Result<()> {
        dotenv().ok();
        let graceful_shutdown_timeout = tokio::time::Duration::from_secs(5);

        let db = make_db_from_env().await.expect("Failed to make db");
        let db = Arc::new(db);

        let mut scheduler = JobScheduler::new().await.expect("Could not create scheduler");
        info!("Starting jobs");
        let jobs = run_jobs(&mut scheduler, db).await.expect("Could not run jobs");

        // Wait for shutdown signal
        shutdown_signal_with_handler(|| async {
            let stop_time = tokio::time::Instant::now();
            stop_jobs(&mut scheduler, jobs, graceful_shutdown_timeout)
                .await
                .expect("Could not stop jobs");
            info!("Jobs stopped in {:?}ms", stop_time.elapsed().as_millis());
        })
        .await;

        Ok(())
    }
}
