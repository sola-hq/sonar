use tracing_otel_extra::init_logging;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let name = env!("CARGO_PKG_NAME");
    init_logging(name).expect("Failed to initialize logging");

    let _ = sonar_api::init_api().await;
}
