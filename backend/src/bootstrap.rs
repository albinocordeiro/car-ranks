use anyhow::{Context, Result};
use tracing::info;

/// Execute full backend bootstrap: initialize tracing, wire state/router, and serve HTTP.
pub(crate) async fn run() -> Result<()> {
    crate::runtime_env::init_tracing();

    let app_state = crate::app_state_builder::build_app_state().await?;
    let app = crate::routes::build_router(app_state);
    let addr = crate::runtime_env::bind_addr_from_env()?;

    info!("backend listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind listener")?;

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
