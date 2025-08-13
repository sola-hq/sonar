use crate::{
    datasource::build_pipeline,
    handlers::health,
    shutdown::shutdown_signal_with_handler,
    ws::{on_connect, IoProxy},
};
use anyhow::{Context, Result};
use axum::{routing::get, Router};
use carbon_core::datasource::Datasource;
use socketioxide::SocketIo;
use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};
use tokio::net::TcpListener;
use tracing::{error, info};

#[derive(Clone, Default)]
pub struct App;

impl App {
    pub fn new() -> Self {
        Self
    }

    pub fn get_port(&self) -> Result<u16> {
        let port = std::env::var("PORT")
            .context("PORT environment variable is not set")?
            .parse::<u16>()
            .context("Failed to parse PORT as a number")?;

        Ok(port)
    }

    pub async fn run<DS>(&self, datasources: Vec<DS>) -> Result<()>
    where
        DS: Datasource + Send + Sync + 'static,
    {
        let port = self.get_port()?;
        let addr = format!("0.0.0.0:{port}");

        let (layer, io) = SocketIo::new_layer();
        io.ns("/", on_connect);

        let io_proxy = IoProxy::new(Arc::new(io), None);
        let app = Router::new().layer(layer).route("/health", get(health::get_health));

        let mut pipeline = build_pipeline(datasources, Arc::new(io_proxy))?;

        // Spawn pipeline in background
        tokio::spawn(async move {
            if let Err(e) = pipeline.run().await {
                error!("Pipeline execution failed: {e}");
            }
        });

        self.start_http_server(app, &addr).await?;

        Ok(())
    }

    async fn start_http_server(&self, app: Router, addr: &str) -> Result<()> {
        let socket_addr = SocketAddr::from_str(addr)
            .context(format!("Failed to parse socket address: {}", addr))?;

        let listener = TcpListener::bind(socket_addr)
            .await
            .context(format!("Failed to bind to address: {}", addr))?;

        info!("Starting HTTP server on {}", addr);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal_with_handler(|| async move {
                info!("Received shutdown signal at {:?}", chrono::Utc::now());
            }))
            .await?;

        info!("HTTP server stopped on {}", addr);
        Ok(())
    }
}
