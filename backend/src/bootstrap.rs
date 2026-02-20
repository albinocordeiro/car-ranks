use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

/// Execute full backend bootstrap: initialize tracing, wire state/router, and serve HTTP.
pub(crate) async fn run() -> Result<()> {
    init_tracing();

    let app_state = build_app_state().await?;
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

/// Build application state by loading signal metadata and establishing DB pools.
async fn build_app_state() -> Result<crate::AppState> {
    let signal_keys =
        Arc::new(crate::load_signal_keys().context("failed to load signal registry v0.2")?);
    info!(
        "locked KPI catalog loaded with {} metric definitions",
        crate::kpi_specs::locked_kpi_catalog_len()
    );

    let (backend, sqlite_pool, pg_pool) = crate::db_bootstrap::initialize_database().await?;

    Ok(crate::AppState {
        sqlite_pool,
        pg_pool,
        backend,
        signal_keys,
    })
}

/// Parse the configured bind address from environment.
fn bind_addr_from_env() -> Result<SocketAddr> {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    bind_addr.parse().context("invalid BIND_ADDR")
}
