use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::{get, post};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};
use tower_http::trace::TraceLayer;
use tracing::info;

mod config;
mod errors;
mod handlers;
mod ingest;
mod jobs;
mod kpi_specs;
mod kpis;
mod metrics;
mod migrations;
mod models;
mod rankings;
mod signals;
mod state;
mod utils;

use errors::ApiError;
pub(crate) use errors::postgres_rollout_not_enabled;
#[cfg(test)]
pub(crate) use handlers::fetch_latest_vehicle_kpis_postgres;
pub(crate) use handlers::{
    get_config_sampling, get_kpis_charging, get_kpis_me, get_kpis_temperature_impact, get_rankings,
    health, post_build_rankings, post_recompute_kpis, post_telemetry_batches,
};
pub(crate) use models::*;
pub(crate) use signals::{load_signal_keys, map_session_event};
pub(crate) use state::{AppState, DatabaseBackend};
pub(crate) use utils::{
    cmp_f64_desc, derive_temperature_bin, normalize_charger_type, now_str, parse_ts,
    percentile_rank, read_positive_env, read_positive_env_f64, timeframe_cutoff,
    timestamp_in_capture_window, year_band,
};

#[cfg(test)]
const INGEST_SCHEMA_VERSION: &str = ingest::INGEST_SCHEMA_VERSION;
#[cfg(test)]
const SQLITE_MIGRATION_0001: &str = migrations::SQLITE_MIGRATION_0001;
#[cfg(test)]
const LEGACY_SQLITE_SCHEMA: &str = migrations::LEGACY_SQLITE_SCHEMA;
#[cfg(test)]
const POSTGRES_MIGRATION_0001: &str = migrations::POSTGRES_MIGRATION_0001;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".to_string()),
        )
        .init();

    let signal_keys = Arc::new(load_signal_keys().context("failed to load signal registry v0.2")?);
    info!(
        "locked KPI catalog loaded with {} metric definitions",
        kpi_specs::locked_kpi_catalog_len()
    );

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://car_ranks.db".to_string());
    let backend =
        if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            DatabaseBackend::Postgres
        } else {
            DatabaseBackend::Sqlite
        };

    let (sqlite_pool, pg_pool) = match backend {
        DatabaseBackend::Sqlite => {
            let connect_options = SqliteConnectOptions::from_str(&database_url)
                .context("invalid sqlite DATABASE_URL")?
                .create_if_missing(true)
                .foreign_keys(true);

            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(10)
                .connect_with(connect_options)
                .await
                .context("failed to connect sqlite")?;
            apply_schema(&sqlite_pool).await?;
            (sqlite_pool, None)
        }
        DatabaseBackend::Postgres => {
            let pg_pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(&database_url)
                .await
                .context("failed to connect postgres")?;
            apply_postgres_schema(&pg_pool).await?;

            // Keep sqlite-only code paths available while postgres rollout is incremental.
            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .context("failed to create sqlite fallback pool")?;
            apply_schema(&sqlite_pool).await?;
            (sqlite_pool, Some(pg_pool))
        }
    };

    let app_state = AppState {
        sqlite_pool,
        pg_pool,
        backend,
        signal_keys,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/config/sampling", get(get_config_sampling))
        .route("/v1/telemetry/batches", post(post_telemetry_batches))
        .route("/v1/kpis/me", get(get_kpis_me))
        .route("/v1/kpis/charging", get(get_kpis_charging))
        .route(
            "/v1/kpis/temperature-impact",
            get(get_kpis_temperature_impact),
        )
        .route("/v1/rankings", get(get_rankings))
        .route("/internal/jobs/recompute-kpis", post(post_recompute_kpis))
        .route(
            "/internal/jobs/build-ranking-snapshots",
            post(post_build_rankings),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let addr: SocketAddr = bind_addr.parse().context("invalid BIND_ADDR")?;

    info!("backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind listener")?;

    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}

async fn run_kpi_job(pool: &SqlitePool) -> Result<JobResponse, ApiError> {
    // Delegate job orchestration to jobs.rs to keep main focused on routing/startup wiring.
    jobs::run_kpi_job(pool).await
}

#[cfg(test)]
async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    jobs::recompute_temperature_kpis(pool).await
}

#[cfg(test)]
async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    jobs::rebuild_temperature_rankings(pool).await
}

async fn apply_schema(pool: &SqlitePool) -> Result<()> {
    // Keep schema orchestration in migrations.rs; main only coordinates startup flow.
    migrations::apply_schema(pool).await
}

async fn apply_postgres_schema(pool: &PgPool) -> Result<()> {
    // Postgres migration tracking mirrors sqlite migration semantics.
    migrations::apply_postgres_schema(pool).await
}

#[cfg(test)]
mod tests;
