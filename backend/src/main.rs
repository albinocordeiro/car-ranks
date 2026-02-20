use anyhow::Result;

mod bootstrap;
mod config;
mod db_bootstrap;
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
mod routes;
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
pub(crate) use jobs::run_kpi_job;
#[cfg(test)]
pub(crate) use jobs::{rebuild_temperature_rankings, recompute_temperature_kpis};
pub(crate) use migrations::{apply_postgres_schema, apply_schema};
pub(crate) use models::*;
pub(crate) use signals::{load_signal_keys, map_session_event};
pub(crate) use state::{AppState, DatabaseBackend};
pub(crate) use utils::{
    cmp_f64_desc, derive_temperature_bin, normalize_charger_type, now_str, parse_ts,
    percentile_rank, read_positive_env, read_positive_env_f64, timeframe_cutoff,
    timestamp_in_capture_window, year_band,
};

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap::run().await
}

#[cfg(test)]
mod tests;
