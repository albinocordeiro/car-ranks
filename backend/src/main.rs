use std::collections::{BTreeMap, HashSet};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
#[cfg(test)]
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
#[cfg(test)]
use chrono::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

mod errors;
mod ingest;
mod jobs;
mod kpi_specs;
mod kpis;
mod metrics;
mod migrations;
mod rankings;
mod state;
mod utils;

use errors::{ApiError, postgres_rollout_not_enabled};
pub(crate) use state::{AppState, DatabaseBackend};
pub(crate) use utils::{
    cmp_f64_desc, derive_temperature_bin, normalize_charger_type, now_str, parse_ts,
    percentile_rank, read_positive_env, read_positive_env_f64, timeframe_cutoff,
    timestamp_in_capture_window, year_band,
};

#[derive(Debug, Deserialize)]
struct TelemetryBatchRequest {
    batch_id: Uuid,
    schema_version: String,
    vehicle_uid: Uuid,
    source: String,
    client: Option<ClientInfo>,
    capture_window: CaptureWindow,
    #[serde(default)]
    records: Vec<TelemetryRecord>,
    #[serde(default)]
    session_events: Vec<SessionEventInput>,
    #[serde(default)]
    diagnostics: Vec<DiagnosticInput>,
}

#[derive(Debug, Deserialize)]
struct ClientInfo {
    platform: Option<String>,
    app_version: Option<String>,
    adapter_fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CaptureWindow {
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    sample_interval_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TelemetryRecord {
    observed_at: DateTime<Utc>,
    signal_key: String,
    value_number: Option<f64>,
    value_string: Option<String>,
    value_bool: Option<bool>,
    value_json: Option<Value>,
    unit: Option<String>,
    status: String,
    confidence: Option<f64>,
    source_signal: Option<String>,
    freshness_ttl_seconds: Option<i64>,
    temperature_bin: Option<String>,
    is_temperature_estimated: Option<bool>,
    session_id: Option<Uuid>,
    raw_payload_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionEventInput {
    event_type: String,
    observed_at: DateTime<Utc>,
    session_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct DiagnosticInput {
    observed_at: DateTime<Utc>,
    mil_on: Option<bool>,
    dtcs_active: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct IngestRecordError {
    record_index: usize,
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    accepted: bool,
    batch_id: Uuid,
    ingest_id: Uuid,
    duplicate: bool,
    records_received: usize,
    records_accepted: usize,
    records_rejected: usize,
    errors: Vec<IngestRecordError>,
    next_upload_after_seconds: i64,
}

#[derive(Debug, Serialize)]
struct SamplingConfigResponse {
    generated_at: String,
    platform: String,
    source: String,
    read_only: bool,
    batch_upload: BatchUploadConfig,
    sampling_profiles: Vec<SamplingProfile>,
    kpi_refresh: KpiRefreshConfig,
    feature_flags: FeatureFlags,
}

#[derive(Debug, Serialize)]
struct BatchUploadConfig {
    default_interval_seconds: i64,
    min_interval_seconds: i64,
    max_interval_seconds: i64,
    next_upload_after_seconds: i64,
}

#[derive(Debug, Serialize)]
struct SamplingProfile {
    mode: String,
    sample_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
struct KpiRefreshConfig {
    active_vehicle_interval_seconds: i64,
    daily_rebuild_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
struct FeatureFlags {
    smartcar_enabled: bool,
    remote_commands_enabled: bool,
}

#[derive(Debug, Serialize)]
struct JobResponse {
    ok: bool,
    job_id: String,
    charging_sessions_upserted: usize,
    kpi_rows_upserted: usize,
    ranking_rows_upserted: usize,
    recomputed_vehicles: usize,
}

#[derive(Debug, Deserialize)]
struct KpiTempQuery {
    vehicle_uid: Uuid,
    timeframe: Option<String>,
    baseline_temperature_bin: Option<String>,
    compare_temperature_bin: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KpiQuery {
    vehicle_uid: Uuid,
    timeframe: Option<String>,
    temperature_bin: Option<String>,
    charger_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct KpiMetric {
    pub(crate) kpi_key: String,
    pub(crate) value: f64,
    pub(crate) unit: String,
    pub(crate) direction: String,
    pub(crate) confidence_level: String,
    pub(crate) sample_count: i64,
}

#[derive(Debug, Serialize)]
struct CohortBenchmark {
    cohort_size: usize,
    percentiles: BTreeMap<String, i64>,
}

#[derive(Debug, Serialize)]
struct TemperatureImpactResponse {
    vehicle_uid: Uuid,
    generated_at: String,
    baseline_temperature_bin: String,
    compare_temperature_bin: String,
    metrics: Vec<KpiMetric>,
    cohort_benchmark: CohortBenchmark,
}

#[derive(Debug, Serialize)]
struct GenericKpiResponse {
    vehicle_uid: Uuid,
    generated_at: String,
    timeframe: String,
    temperature_bin: String,
    ranking_type: String,
    kpis: Vec<KpiMetric>,
}

#[derive(Debug, Deserialize)]
struct RankingsQuery {
    ranking_type: String,
    timeframe: Option<String>,
    temperature_bin: Option<String>,
    powertrain_class: Option<String>,
    make: Option<String>,
    model: Option<String>,
    trim: Option<String>,
    year_band: Option<String>,
    region: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
struct RankingRow {
    rank: i64,
    vehicle_uid: Uuid,
    score: f64,
    confidence_level: String,
    kpis: BTreeMap<String, f64>,
}

#[derive(Debug, Serialize)]
struct RankingPage {
    limit: i64,
    offset: i64,
    has_more: bool,
}

#[derive(Debug, Serialize)]
struct RankingCohort {
    cohort_key: String,
    cohort_size: i64,
    sample_gate_passed: bool,
}

#[derive(Debug, Serialize)]
struct RankingsResponse {
    generated_at: String,
    ranking_type: String,
    timeframe: String,
    temperature_bin: String,
    filters: BTreeMap<String, Option<String>>,
    cohort: RankingCohort,
    rows: Vec<RankingRow>,
    page: RankingPage,
}

#[derive(Debug)]
struct MetricCalc {
    pub(crate) key: &'static str,
    pub(crate) value: f64,
    pub(crate) unit: &'static str,
    pub(crate) direction: &'static str,
    pub(crate) sample_count: i64,
    pub(crate) confidence_level: &'static str,
}

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

async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "car-ranks-backend",
        "timestamp": now_str()
    }))
}

async fn get_config_sampling() -> Json<SamplingConfigResponse> {
    let min_interval_seconds = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MIN_SECONDS", 60);
    let max_interval_candidate = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MAX_SECONDS", 86_400);
    let max_interval_seconds = max_interval_candidate.max(min_interval_seconds);
    let default_interval_seconds = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_SECONDS", 60)
        .clamp(min_interval_seconds, max_interval_seconds);

    let driving_sample_interval_seconds =
        read_positive_env("CAR_RANKS_DRIVING_SAMPLE_INTERVAL_SECONDS", 5);
    let charging_sample_interval_seconds =
        read_positive_env("CAR_RANKS_CHARGING_SAMPLE_INTERVAL_SECONDS", 10);
    let parked_sample_interval_seconds =
        read_positive_env("CAR_RANKS_PARKED_SAMPLE_INTERVAL_SECONDS", 60);

    let active_vehicle_interval_seconds =
        read_positive_env("CAR_RANKS_ACTIVE_KPI_REFRESH_SECONDS", 300);
    let daily_rebuild_interval_seconds =
        read_positive_env("CAR_RANKS_DAILY_REBUILD_SECONDS", 86_400);

    Json(SamplingConfigResponse {
        generated_at: now_str(),
        platform: "ios".to_string(),
        source: "obd".to_string(),
        read_only: true,
        batch_upload: BatchUploadConfig {
            default_interval_seconds,
            min_interval_seconds,
            max_interval_seconds,
            next_upload_after_seconds: default_interval_seconds,
        },
        sampling_profiles: vec![
            SamplingProfile {
                mode: "driving".to_string(),
                sample_interval_seconds: driving_sample_interval_seconds,
            },
            SamplingProfile {
                mode: "charging".to_string(),
                sample_interval_seconds: charging_sample_interval_seconds,
            },
            SamplingProfile {
                mode: "parked".to_string(),
                sample_interval_seconds: parked_sample_interval_seconds,
            },
        ],
        kpi_refresh: KpiRefreshConfig {
            active_vehicle_interval_seconds,
            daily_rebuild_interval_seconds,
        },
        feature_flags: FeatureFlags {
            smartcar_enabled: false,
            remote_commands_enabled: false,
        },
    })
}

async fn post_telemetry_batches(
    State(state): State<AppState>,
    Json(payload): Json<TelemetryBatchRequest>,
) -> Result<Json<IngestResponse>, ApiError> {
    // Keep router-facing handlers thin; ingestion rules and persistence live in ingest.rs.
    ingest::post_telemetry_batches(State(state), Json(payload)).await
}

async fn post_recompute_kpis(State(state): State<AppState>) -> Result<Json<JobResponse>, ApiError> {
    if state.backend != DatabaseBackend::Sqlite {
        return Err(postgres_rollout_not_enabled(
            "/internal/jobs/recompute-kpis",
        ));
    }
    run_kpi_job(&state.sqlite_pool).await.map(Json)
}

async fn post_build_rankings(State(state): State<AppState>) -> Result<Json<JobResponse>, ApiError> {
    if state.backend != DatabaseBackend::Sqlite {
        return Err(postgres_rollout_not_enabled(
            "/internal/jobs/build-ranking-snapshots",
        ));
    }
    run_kpi_job(&state.sqlite_pool).await.map(Json)
}

async fn get_kpis_me(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    // KPI reads are delegated to kpis.rs so backend-specific query logic is isolated.
    kpis::get_kpis_me(State(state), Query(params)).await
}

async fn get_kpis_charging(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    // Keep endpoint wiring stable while charging KPI behavior evolves in one module.
    kpis::get_kpis_charging(State(state), Query(params)).await
}

async fn get_kpis_temperature_impact(
    State(state): State<AppState>,
    Query(params): Query<KpiTempQuery>,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    // Temperature KPI aggregation and cohort percentile logic are centralized in kpis.rs.
    kpis::get_kpis_temperature_impact(State(state), Query(params)).await
}

#[cfg(test)]
async fn fetch_latest_vehicle_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>> {
    // Postgres-specific KPI fetch path is only needed in tests for now.
    kpis::fetch_latest_vehicle_kpis_postgres(
        pool,
        vehicle_uid,
        ranking_type,
        timeframe,
        temperature_bin,
    )
    .await
}

async fn get_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    // Ranking query construction and row materialization live in rankings.rs.
    rankings::get_rankings(State(state), Query(params)).await
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

fn load_signal_keys() -> Result<HashSet<String>> {
    let raw = include_str!("../../research/schema/signal_registry_v0_2.json");
    let value: Value = serde_json::from_str(raw).context("invalid JSON in signal_registry_v0_2")?;

    let signals = value
        .get("signals")
        .and_then(Value::as_array)
        .context("signals array missing in signal_registry_v0_2")?;

    let mut keys = HashSet::new();
    for signal in signals {
        if let Some(key) = signal.get("signal_key").and_then(Value::as_str) {
            keys.insert(key.to_string());
        }
    }

    Ok(keys)
}

fn map_session_event(event_type: &str) -> Option<(&'static str, &'static str)> {
    match event_type {
        "drive_session_start" => Some(("drive", "start")),
        "drive_session_stop" => Some(("drive", "stop")),
        "charging_session_start" => Some(("charging", "start")),
        "charging_session_stop" => Some(("charging", "stop")),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
