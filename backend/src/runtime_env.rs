use std::net::SocketAddr;

use anyhow::{Context, Result};

/// Initialize process tracing with environment override support.
pub(crate) fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".to_string()),
        )
        .init();
}

/// Parse the configured HTTP bind address from environment.
pub(crate) fn bind_addr_from_env() -> Result<SocketAddr> {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    bind_addr.parse().context("invalid BIND_ADDR")
}
