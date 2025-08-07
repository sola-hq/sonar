use tracing::error;

#[tokio::main]
async fn main() {
    if let Err(err) = sonar::cli::run().await {
        error!("Error: {err:?}");
        std::process::exit(1);
    }
    println!("Done");
}
