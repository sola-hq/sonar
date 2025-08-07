use chrono::Utc;
use sonar_db::make_db_from_env;
use sonar_scheduler::{
    job::{run_jobs, stop_jobs},
    shutdown_signal_with_handler,
};
use std::{env, sync::Arc};
use tokio_cron_scheduler::JobScheduler;
use tracing::info;
use tracing_otel_extra::init_logging;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_logging(env!("CARGO_PKG_NAME")).expect("Failed to initialize logging");
    let graceful_shutdown_timeout = tokio::time::Duration::from_secs(10);

    let db = make_db_from_env().await.expect("Failed to make db");
    let db = Arc::new(db);

    let mut scheduler = JobScheduler::new().await.expect("Could not create scheduler");
    info!("Starting jobs");
    let jobs = run_jobs(&mut scheduler, db).await.expect("Could not run jobs");

    // Wait for shutdown signal
    shutdown_signal_with_handler(|| async {
        let stop_time = Utc::now();
        stop_jobs(&mut scheduler, jobs, graceful_shutdown_timeout)
            .await
            .expect("Could not stop jobs");
        info!(
            "Jobs stopped in {:?}ms",
            Utc::now().signed_duration_since(stop_time).num_milliseconds()
        );
    })
    .await;
}
