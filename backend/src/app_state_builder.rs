use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

/// Build the shared application state used by all HTTP handlers.
///
/// This keeps state assembly (signal registry + database resources) separate
/// from process lifecycle orchestration in `bootstrap.rs`.
pub(crate) async fn build_app_state() -> Result<crate::AppState> {
    let signal_keys =
        Arc::new(crate::load_signal_keys().context("failed to load signal registry v0.2")?);
    info!(
        "locked KPI catalog loaded with {} metric definitions",
        crate::kpi_specs::locked_kpi_catalog_len()
    );

    let pg_pool = crate::db_bootstrap::initialize_database().await?;

    Ok(crate::AppState {
        pg_pool,
        signal_keys,
    })
}
