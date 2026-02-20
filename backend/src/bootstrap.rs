use std::net::SocketAddr;

use anyhow::{Context, Result};
use tracing::info;

/// Execute full backend bootstrap: initialize tracing, wire state/router, and serve HTTP.
pub(crate) async fn run() -> Result<()> {
    init_tracing();

    let app_state = crate::app_state_builder::build_app_state().await?;
    let app = crate::routes::build_router(app_state);
    let addr = bind_addr_from_env()?;

    info!("backend listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind listener")?;

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

/// Initialize tracing once at process startup, with environment override support.
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".to_string()),
        )
        .init();
}

/// Parse the configured bind address from environment.
fn bind_addr_from_env() -> Result<SocketAddr> {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    bind_addr.parse().context("invalid BIND_ADDR")
}
