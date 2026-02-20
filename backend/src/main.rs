use anyhow::Result;

mod app_state_builder;
mod auth;
mod bootstrap;
mod config;
mod db_bootstrap;
mod errors;
mod handlers;
mod ingest;
mod job_runs;
mod jobs;
mod kpi_specs;
mod kpis;
mod metrics;
mod migrations;
mod models;
mod rankings;
mod routes;
mod runtime_env;
mod signals;
mod state;
mod utils;

pub(crate) use errors::ApiError;
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
