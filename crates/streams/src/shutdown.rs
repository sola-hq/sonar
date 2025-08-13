//! This module provides utilities for handling graceful shutdown signals.
//!
//! The code is adapted from the `axum-server` crate:
//! <https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs>

use tokio::signal;

/// Waits for a shutdown signal.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
    }
}

/// Waits for a shutdown signal and then invokes a handler.
pub async fn shutdown_signal_with_handler<F, Fut>(shutdown_handler: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // Wait for a shutdown signal
    shutdown_signal().await;

    // Invoke the shutdown handler
    shutdown_handler().await;
}
