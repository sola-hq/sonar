use crate::{
    datasource::{
        account::{
            make_system_account_subscribe_datasource, make_token_2022_account_subscribe_datasource,
            make_token_account_subscribe_datasource,
        },
        build_pipeline,
    },
    handlers::health,
    shutdown::shutdown_signal_with_handler,
    ws::{on_connect, IoProxy},
};
use anyhow::{Context, Result};
use axum::{routing::get, Router};
use carbon_core::pipeline::Pipeline;
use socketioxide::{adapter::Adapter, SocketIo};
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

    pub fn get_port(&self) -> u16 {
        std::env::var("PORT")
            .expect("Expected PORT to be set")
            .parse()
            .expect("Expected PORT to be a number")
    }

    fn get_pipeline<A: Adapter>(&self, io_proxy: Arc<IoProxy<A>>) -> Result<Pipeline> {
        let datasource = vec![
            make_token_account_subscribe_datasource(),
            make_token_2022_account_subscribe_datasource(),
            make_system_account_subscribe_datasource(),
        ];
        let pipeline = build_pipeline(datasource, io_proxy).context("Failed to build pipeline")?;
        Ok(pipeline)
    }

    pub async fn run(&self) -> Result<()> {
        // let adapter_ctor = init_adapter().await.expect("Failed to create RedisAdapter");

        // let (socket_layer, io) =
        //     SocketIo::builder().with_adapter::<RedisAdapter<_>>(adapter_ctor).build_layer();
        let (layer, io) = SocketIo::new_layer();

        io.ns("/", on_connect);

        let io_proxy = IoProxy::new(Arc::new(io), None);

        let app = Router::new().layer(layer).route("/health", get(health::get_health));

        let mut pipeline = self.get_pipeline(Arc::new(io_proxy))?;

        tokio::spawn(async move {
            if let Err(e) = pipeline.run().await {
                error!("Failed to run pipeline: {e}");
            }
        });

        self.start_http_server(app, &format!("0.0.0.0:{}", self.get_port())).await?;

        Ok(())
    }

    async fn start_http_server(&self, app: Router, addr: &str) -> Result<()> {
        let socket_addr =
            SocketAddr::from_str(addr).context(format!("Failed to socket address {addr}"))?;

        let listener = TcpListener::bind(socket_addr)
            .await
            .context(format!("Failed to bind to address {addr}"))?;

        tracing::info!("Starting HTTP server on {}", addr);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal_with_handler(|| async move {
                info!("Received shutdown signal at {:?}", chrono::Utc::now());
            }))
            .await?;
        tracing::info!("HTTP server on {}", addr);
        Ok(())
    }
}
