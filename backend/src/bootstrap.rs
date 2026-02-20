use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::{get, post};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower_http::trace::TraceLayer;
use tracing::info;

/// Execute full backend bootstrap: initialize tracing, wire state/router, and serve HTTP.
pub(crate) async fn run() -> Result<()> {
    init_tracing();

    let app_state = build_app_state().await?;
    let app = build_router(app_state);
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

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://car_ranks.db".to_string());
    let backend =
        if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            crate::DatabaseBackend::Postgres
        } else {
            crate::DatabaseBackend::Sqlite
        };

    let (sqlite_pool, pg_pool) = match backend {
        crate::DatabaseBackend::Sqlite => {
            let connect_options = SqliteConnectOptions::from_str(&database_url)
                .context("invalid sqlite DATABASE_URL")?
                .create_if_missing(true)
                .foreign_keys(true);

            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(10)
                .connect_with(connect_options)
                .await
                .context("failed to connect sqlite")?;
            crate::apply_schema(&sqlite_pool).await?;
            (sqlite_pool, None)
        }
        crate::DatabaseBackend::Postgres => {
            let pg_pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(&database_url)
                .await
                .context("failed to connect postgres")?;
            crate::apply_postgres_schema(&pg_pool).await?;

            // Keep sqlite-only code paths available while postgres rollout is incremental.
            let sqlite_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .context("failed to create sqlite fallback pool")?;
            crate::apply_schema(&sqlite_pool).await?;
            (sqlite_pool, Some(pg_pool))
        }
    };

    Ok(crate::AppState {
        sqlite_pool,
        pg_pool,
        backend,
        signal_keys,
    })
}

/// Construct the full HTTP router with all public/internal routes and middleware.
fn build_router(app_state: crate::AppState) -> Router {
    Router::new()
        .route("/health", get(crate::health))
        .route("/v1/config/sampling", get(crate::get_config_sampling))
        .route("/v1/telemetry/batches", post(crate::post_telemetry_batches))
        .route("/v1/kpis/me", get(crate::get_kpis_me))
        .route("/v1/kpis/charging", get(crate::get_kpis_charging))
        .route(
            "/v1/kpis/temperature-impact",
            get(crate::get_kpis_temperature_impact),
        )
        .route("/v1/rankings", get(crate::get_rankings))
        .route(
            "/internal/jobs/recompute-kpis",
            post(crate::post_recompute_kpis),
        )
        .route(
            "/internal/jobs/build-ranking-snapshots",
            post(crate::post_build_rankings),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}

/// Parse the configured bind address from environment.
fn bind_addr_from_env() -> Result<SocketAddr> {
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    bind_addr.parse().context("invalid BIND_ADDR")
}
