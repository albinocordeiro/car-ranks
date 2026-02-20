use std::cmp::Ordering;
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
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, Row, SqlitePool};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

mod errors;
mod ingest;
mod jobs;
mod kpis;
mod metrics;
mod migrations;
mod rankings;

use errors::{ApiError, postgres_rollout_not_enabled};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DatabaseBackend {
    Sqlite,
    Postgres,
}

#[derive(Clone)]
struct AppState {
    sqlite_pool: SqlitePool,
    pg_pool: Option<PgPool>,
    backend: DatabaseBackend,
    signal_keys: Arc<HashSet<String>>,
}

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
struct VehiclePoint {
    temperature_c: f64,
    km_per_soc: f64,
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

#[derive(Debug)]
struct LockedKpiSpec {
    ranking_type: &'static str,
    kpi_key: &'static str,
    formula: &'static str,
    required_signals: &'static [&'static str],
    optional_signals: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
struct TemperatureSampleGates {
    min_cold_distance_km: f64,
    min_mild_distance_km: f64,
    min_cold_charge_sessions: usize,
    min_mild_charge_sessions: usize,
    min_sensitivity_points: usize,
}

impl TemperatureSampleGates {
    fn range_gate_passed(self, cold_distance_km: f64, mild_distance_km: f64) -> bool {
        cold_distance_km >= self.min_cold_distance_km
            && mild_distance_km >= self.min_mild_distance_km
    }

    fn charge_gate_passed(self, cold_sessions: usize, mild_sessions: usize) -> bool {
        cold_sessions >= self.min_cold_charge_sessions
            && mild_sessions >= self.min_mild_charge_sessions
    }
}

const LOCKED_KPI_SPECS: &[LockedKpiSpec] = &[
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "ev_net_energy_efficiency",
        formula: "median(((delta_soc_pct/100) * DEFAULT_USABLE_BATTERY_KWH * 1000) / delta_km)",
        required_signals: &["distance.odometer", "ev.soc_pct"],
        optional_signals: &["power.battery_power_kw", "environment.ambient_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "ev_estimated_practical_range",
        formula: "latest_soc_pct * median(delta_km / delta_soc_pct)",
        required_signals: &["distance.odometer", "ev.soc_pct"],
        optional_signals: &["environment.ambient_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "ev_urban_efficiency",
        formula: "median(ev_net_energy_efficiency for segments where speed.vehicle < 45 km/h)",
        required_signals: &["distance.odometer", "ev.soc_pct", "speed.vehicle"],
        optional_signals: &[],
    },
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "ev_highway_efficiency",
        formula: "median(ev_net_energy_efficiency for segments where speed.vehicle >= 80 km/h)",
        required_signals: &["distance.odometer", "ev.soc_pct", "speed.vehicle"],
        optional_signals: &[],
    },
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "regeneration_recovery_ratio",
        formula: "100 * regen_energy_wh / (regen_energy_wh + traction_energy_wh) over integrated power windows",
        required_signals: &["ev.regen_power_kw", "ev.traction_power_kw"],
        optional_signals: &[],
    },
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "soc_depletion_rate_per_100km",
        formula: "100 / median(delta_km / delta_soc_pct)",
        required_signals: &["distance.odometer", "ev.soc_pct"],
        optional_signals: &[],
    },
    LockedKpiSpec {
        ranking_type: "ev_range_efficiency",
        kpi_key: "ev_range_efficiency_score",
        formula: "0.65 * normalized_efficiency_component + 0.35 * normalized_estimated_range_component",
        required_signals: &["distance.odometer", "ev.soc_pct"],
        optional_signals: &["speed.vehicle", "environment.ambient_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_charging_performance",
        kpi_key: "temp_adjusted_charge_acceptance_score",
        formula: "clamp(100 * median(all_charge_kw) / median(mild_charge_kw), 0, 120)",
        required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
        optional_signals: &[
            "ev.battery_temp_c",
            "environment.ambient_temp_c",
            "ev.charger_type",
        ],
    },
    LockedKpiSpec {
        ranking_type: "ev_charging_performance",
        kpi_key: "cold_weather_charge_speed_retention",
        formula: "100 * median(cold_charge_kw) / median(mild_charge_kw)",
        required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
        optional_signals: &[
            "ev.battery_temp_c",
            "environment.ambient_temp_c",
            "ev.charger_type",
        ],
    },
    LockedKpiSpec {
        ranking_type: "ev_charging_performance",
        kpi_key: "charging_performance_score",
        formula: "0.6 * temp_adjusted_charge_acceptance_score + 0.4 * cold_weather_charge_speed_retention",
        required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
        optional_signals: &["ev.battery_temp_c", "environment.ambient_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_temperature_impact",
        kpi_key: "cold_weather_range_retention",
        formula: "100 * median(cold_km_per_soc) / median(mild_km_per_soc)",
        required_signals: &[
            "distance.odometer",
            "ev.soc_pct",
            "environment.ambient_temp_c",
        ],
        optional_signals: &["ev.battery_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_temperature_impact",
        kpi_key: "range_temperature_sensitivity_index",
        formula: "max(0, -slope(km_per_soc vs temp_c) * 10 / mild_km_per_soc * 100)",
        required_signals: &[
            "distance.odometer",
            "ev.soc_pct",
            "environment.ambient_temp_c",
        ],
        optional_signals: &["ev.battery_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_temperature_impact",
        kpi_key: "cold_weather_charge_speed_retention",
        formula: "100 * median(cold_charge_kw) / median(mild_charge_kw)",
        required_signals: &["ev.charging_state", "ev.charge_power_kw", "ev.soc_pct"],
        optional_signals: &["ev.battery_temp_c", "environment.ambient_temp_c"],
    },
    LockedKpiSpec {
        ranking_type: "ev_composite",
        kpi_key: "ev_composite_base_score",
        formula: "0.6 * ev_range_efficiency_score + 0.4 * charging_performance_score",
        required_signals: &[],
        optional_signals: &[],
    },
    LockedKpiSpec {
        ranking_type: "ev_composite",
        kpi_key: "ev_health_modifier_penalty",
        formula: "min(10, (MIL_ON ? 6 : 0) + min(4, 0.5 * distinct_active_dtc_count))",
        required_signals: &["diag.mil_on", "diag.dtcs_active"],
        optional_signals: &[],
    },
    LockedKpiSpec {
        ranking_type: "ev_composite",
        kpi_key: "ev_composite_score",
        formula: "clamp(ev_composite_base_score - ev_health_modifier_penalty, 0, 100)",
        required_signals: &[],
        optional_signals: &[],
    },
];

#[cfg(test)]
const INGEST_SCHEMA_VERSION: &str = ingest::INGEST_SCHEMA_VERSION;
#[cfg(test)]
const SQLITE_MIGRATION_0001: &str = migrations::SQLITE_MIGRATION_0001;
#[cfg(test)]
const LEGACY_SQLITE_SCHEMA: &str = migrations::LEGACY_SQLITE_SCHEMA;
#[cfg(test)]
const POSTGRES_MIGRATION_0001: &str = migrations::POSTGRES_MIGRATION_0001;

fn lookup_kpi_spec(ranking_type: &str, kpi_key: &str) -> Option<&'static LockedKpiSpec> {
    LOCKED_KPI_SPECS
        .iter()
        .find(|spec| spec.ranking_type == ranking_type && spec.kpi_key == kpi_key)
}

pub(crate) fn locked_kpi_spec_details(
    ranking_type: &str,
    kpi_key: &str,
) -> Option<(
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
)> {
    lookup_kpi_spec(ranking_type, kpi_key)
        .map(|spec| (spec.formula, spec.required_signals, spec.optional_signals))
}

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
        LOCKED_KPI_SPECS.len()
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

pub(crate) async fn compute_range_efficiency_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    metrics::compute_range_efficiency_metrics(pool, vehicle_uid, cutoff).await
}

pub(crate) async fn compute_charging_performance_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = temperature_sample_gates();

    let charge_rows = sqlx::query(
        r#"
        SELECT avg_charge_power_kw, temperature_bin
        FROM vehicle_charging_session
        WHERE vehicle_uid = ?
          AND started_at >= ?
          AND avg_charge_power_kw IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch charging sessions for charging KPIs")?;

    let mut all_power = Vec::new();
    let mut cold_power = Vec::new();
    let mut mild_power = Vec::new();
    for row in charge_rows {
        let power: Option<f64> = row.try_get("avg_charge_power_kw")?;
        let bin: Option<String> = row.try_get("temperature_bin")?;
        if let (Some(p), Some(b)) = (power, bin) {
            if p <= 0.0 || !p.is_finite() {
                continue;
            }
            all_power.push(p);
            if b == "cold" || b == "very_cold" {
                cold_power.push(p);
            }
            if b == "mild" {
                mild_power.push(p);
            }
        }
    }

    if all_power.is_empty() {
        return Ok(Vec::new());
    }

    let mut metrics = Vec::new();
    let sample_count = all_power.len() as i64;
    let all_median = median(all_power.clone()).unwrap_or(0.0);
    let mild_median = median(mild_power.clone()).unwrap_or(all_median.max(1e-6));
    let acceptance_score = if mild_median > 0.0 {
        (100.0 * all_median / mild_median).clamp(0.0, 120.0)
    } else {
        100.0
    };

    metrics.push(MetricCalc {
        key: "temp_adjusted_charge_acceptance_score",
        value: acceptance_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });

    if gates.charge_gate_passed(cold_power.len(), mild_power.len()) {
        if let (Some(cold_median), Some(mild_median)) =
            (median(cold_power.clone()), median(mild_power.clone()))
        {
            if mild_median > 0.0 {
                let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
                let retention_samples = cold_power.len().min(mild_power.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count: retention_samples,
                    confidence_level: confidence_from_samples(retention_samples),
                });
            }
        }
    }

    let charging_score = if let Some(retention_metric) = metrics
        .iter()
        .find(|m| m.key == "cold_weather_charge_speed_retention")
    {
        (0.6 * acceptance_score + 0.4 * retention_metric.value).clamp(0.0, 100.0)
    } else {
        acceptance_score.clamp(0.0, 100.0)
    };

    metrics.push(MetricCalc {
        key: "charging_performance_score",
        value: charging_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });

    Ok(metrics)
}

pub(crate) async fn compute_composite_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
    range_metrics: &[MetricCalc],
    charging_metrics: &[MetricCalc],
) -> Result<Vec<MetricCalc>> {
    let range_score = range_metrics
        .iter()
        .find(|m| m.key == "ev_range_efficiency_score")
        .map(|m| m.value);
    let charging_score = charging_metrics
        .iter()
        .find(|m| m.key == "charging_performance_score")
        .map(|m| m.value);

    let Some(base_composite_score) = (match (range_score, charging_score) {
        (Some(r), Some(c)) => Some((0.6 * r + 0.4 * c).clamp(0.0, 100.0)),
        (Some(r), None) => Some(r.clamp(0.0, 100.0)),
        (None, Some(c)) => Some(c.clamp(0.0, 100.0)),
        (None, None) => None,
    }) else {
        return Ok(Vec::new());
    };

    let (health_penalty, health_sample_count) =
        compute_health_modifier_penalty(pool, vehicle_uid, cutoff).await?;
    let adjusted_score = (base_composite_score - health_penalty).clamp(0.0, 100.0);

    let sample_count = (range_metrics
        .iter()
        .find(|m| m.key == "ev_range_efficiency_score")
        .map(|m| m.sample_count)
        .unwrap_or(0))
    .max(
        charging_metrics
            .iter()
            .find(|m| m.key == "charging_performance_score")
            .map(|m| m.sample_count)
            .unwrap_or(0),
    )
    .max(health_sample_count);

    Ok(vec![
        MetricCalc {
            key: "ev_composite_base_score",
            value: base_composite_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: confidence_from_samples(sample_count),
        },
        MetricCalc {
            key: "ev_health_modifier_penalty",
            value: health_penalty,
            unit: "score_points",
            direction: "lower_is_better",
            sample_count: health_sample_count,
            confidence_level: confidence_from_samples(health_sample_count),
        },
        MetricCalc {
            key: "ev_composite_score",
            value: adjusted_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: confidence_from_samples(sample_count),
        },
    ])
}

pub(crate) async fn compute_health_modifier_penalty(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<(f64, i64)> {
    let dtc_row = sqlx::query(
        r#"
        SELECT COUNT(DISTINCT code) AS dtc_count
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND event_type = 'DTC_ACTIVE'
          AND code IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_one(pool)
    .await
    .context("failed to compute active DTC count for health modifier")?;

    let dtc_count: i64 = dtc_row
        .try_get("dtc_count")
        .context("failed to parse active DTC count")?;

    let mil_row = sqlx::query(
        r#"
        SELECT event_type
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND event_type IN ('MIL_ON', 'MIL_OFF')
        ORDER BY observed_at DESC
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_optional(pool)
    .await
    .context("failed to load MIL status for health modifier")?;

    let mil_event_type = mil_row.and_then(|row| row.try_get::<String, _>("event_type").ok());
    let mil_on = mil_event_type
        .as_deref()
        .map(|event_type| event_type == "MIL_ON")
        .unwrap_or(false);

    let mil_penalty = if mil_on { 6.0 } else { 0.0 };
    let dtc_penalty = (dtc_count.max(0) as f64 * 0.5).min(4.0);
    let penalty = (mil_penalty + dtc_penalty).min(10.0);

    let sample_count = dtc_count.max(0) + if mil_event_type.is_some() { 1 } else { 0 };
    Ok((penalty, sample_count.max(1)))
}

pub(crate) async fn compute_vehicle_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
    let gates = temperature_sample_gates();

    let obs_rows = sqlx::query(
        r#"
        SELECT signal_key, value_number, observed_at
        FROM vehicle_signal_observation
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND signal_key IN ('distance.odometer', 'ev.soc_pct', 'environment.ambient_temp_c')
        ORDER BY observed_at ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch observation rows for KPI computation")?;

    #[derive(Default)]
    struct TimestampSnapshot {
        odo: Option<f64>,
        soc: Option<f64>,
        temp: Option<f64>,
    }

    let mut by_ts: BTreeMap<DateTime<Utc>, TimestampSnapshot> = BTreeMap::new();
    for row in obs_rows {
        let signal_key: String = row.try_get("signal_key")?;
        let value: Option<f64> = row.try_get("value_number")?;
        let observed_at: String = row.try_get("observed_at")?;
        let Some(ts) = parse_ts(&observed_at) else {
            continue;
        };

        let snapshot = by_ts.entry(ts).or_default();
        match (signal_key.as_str(), value) {
            ("distance.odometer", Some(v)) => snapshot.odo = Some(v),
            ("ev.soc_pct", Some(v)) => snapshot.soc = Some(v),
            ("environment.ambient_temp_c", Some(v)) => snapshot.temp = Some(v),
            _ => {}
        }
    }

    let mut current_odo: Option<f64> = None;
    let mut current_soc: Option<f64> = None;
    let mut current_temp: Option<f64> = None;
    let mut prev_filled: Option<(f64, f64, f64)> = None;
    let mut points = Vec::new();
    let mut cold_values = Vec::new();
    let mut mild_values = Vec::new();
    let mut cold_distance_km = 0.0;
    let mut mild_distance_km = 0.0;

    for snapshot in by_ts.values() {
        if snapshot.odo.is_some() {
            current_odo = snapshot.odo;
        }
        if snapshot.soc.is_some() {
            current_soc = snapshot.soc;
        }
        if snapshot.temp.is_some() {
            current_temp = snapshot.temp;
        }

        if let (Some(odo), Some(soc), Some(temp)) = (current_odo, current_soc, current_temp) {
            if let Some((prev_odo, prev_soc, _prev_temp)) = prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0 {
                    let km_per_soc = delta_km / delta_soc;
                    if km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0 {
                        points.push(VehiclePoint {
                            temperature_c: temp,
                            km_per_soc,
                        });
                        if temp <= 5.0 {
                            cold_values.push(km_per_soc);
                            cold_distance_km += delta_km;
                        }
                        if temp > 15.0 && temp <= 25.0 {
                            mild_values.push(km_per_soc);
                            mild_distance_km += delta_km;
                        }
                    }
                }
            }
            prev_filled = Some((odo, soc, temp));
        }
    }

    let mut metrics = Vec::new();

    if gates.range_gate_passed(cold_distance_km, mild_distance_km) {
        let cold_median = median(cold_values.clone());
        let mild_median = median(mild_values.clone());
        if let (Some(cold), Some(mild)) = (cold_median, mild_median) {
            if mild > 0.0 {
                let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
                let sample_count = cold_values.len().min(mild_values.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_range_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count,
                    confidence_level: confidence_from_samples(sample_count),
                });

                if points.len() >= gates.min_sensitivity_points {
                    if let Some(slope) = linear_regression_slope(&points) {
                        let loss_pct_per_10c_drop = if slope < 0.0 {
                            ((-slope * 10.0) / mild) * 100.0
                        } else {
                            0.0
                        }
                        .clamp(0.0, 100.0);

                        metrics.push(MetricCalc {
                            key: "range_temperature_sensitivity_index",
                            value: loss_pct_per_10c_drop,
                            unit: "%_loss_per_10C_drop",
                            direction: "lower_is_better",
                            sample_count: points.len() as i64,
                            confidence_level: confidence_from_samples(points.len() as i64),
                        });
                    }
                }
            }
        }
    }

    let charge_rows = sqlx::query(
        r#"
        SELECT avg_charge_power_kw, temperature_bin
        FROM vehicle_charging_session
        WHERE vehicle_uid = ?
          AND started_at >= ?
          AND avg_charge_power_kw IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_all(pool)
    .await
    .context("failed to fetch charging sessions for KPI computation")?;

    let mut cold_charge = Vec::new();
    let mut mild_charge = Vec::new();

    for row in charge_rows {
        let power: Option<f64> = row.try_get("avg_charge_power_kw")?;
        let bin: Option<String> = row.try_get("temperature_bin")?;

        if let (Some(power), Some(bin)) = (power, bin) {
            if power <= 0.0 || !power.is_finite() {
                continue;
            }
            if bin == "cold" || bin == "very_cold" {
                cold_charge.push(power);
            }
            if bin == "mild" {
                mild_charge.push(power);
            }
        }
    }

    if gates.charge_gate_passed(cold_charge.len(), mild_charge.len()) {
        if let (Some(cold), Some(mild)) = (median(cold_charge.clone()), median(mild_charge.clone()))
        {
            if mild > 0.0 {
                let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
                let sample_count = cold_charge.len().min(mild_charge.len()) as i64;
                metrics.push(MetricCalc {
                    key: "cold_weather_charge_speed_retention",
                    value: retention,
                    unit: "%",
                    direction: "higher_is_better",
                    sample_count,
                    confidence_level: confidence_from_samples(sample_count),
                });
            }
        }
    }

    Ok(metrics)
}

#[cfg(test)]
fn score_from_kpi_map(ranking_type: &str, kpis: &BTreeMap<String, f64>) -> f64 {
    metrics::score_from_kpi_map(ranking_type, kpis)
}

pub(crate) fn confidence_from_kpi_metrics(kpis: &[KpiMetric]) -> &'static str {
    if kpis.is_empty() {
        return "preview";
    }
    if kpis.iter().any(|k| k.confidence_level == "preview") {
        "preview"
    } else if kpis.iter().any(|k| k.confidence_level == "medium") {
        "medium"
    } else {
        "stable"
    }
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

pub(crate) fn derive_temperature_bin(temp_c: f64) -> &'static str {
    if temp_c <= -5.0 {
        "very_cold"
    } else if temp_c <= 5.0 {
        "cold"
    } else if temp_c <= 15.0 {
        "cool"
    } else if temp_c <= 25.0 {
        "mild"
    } else {
        "hot"
    }
}

pub(crate) fn normalize_charger_type(value: &str) -> &'static str {
    let lower = value.to_ascii_lowercase();
    if lower.contains("dc") || lower.contains("fast") {
        "dc"
    } else if lower.contains("ac") || lower.contains("level") {
        "ac"
    } else {
        "unknown"
    }
}

pub(crate) fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn timestamp_in_capture_window(
    observed_at: &DateTime<Utc>,
    started_at: &DateTime<Utc>,
    ended_at: &DateTime<Utc>,
) -> bool {
    observed_at >= started_at && observed_at <= ended_at
}

pub(crate) fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn read_positive_env(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

pub(crate) fn read_positive_env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| *v > 0.0)
        .unwrap_or(default)
}

fn temperature_sample_gates() -> TemperatureSampleGates {
    let min_cold_charge_sessions =
        read_positive_env("CAR_RANKS_TEMP_GATE_MIN_COLD_CHARGE_SESSIONS", 1);
    let min_mild_charge_sessions =
        read_positive_env("CAR_RANKS_TEMP_GATE_MIN_MILD_CHARGE_SESSIONS", 1);
    let min_sensitivity_points = read_positive_env("CAR_RANKS_TEMP_GATE_MIN_SENSITIVITY_POINTS", 6);

    TemperatureSampleGates {
        min_cold_distance_km: read_positive_env_f64(
            "CAR_RANKS_TEMP_GATE_MIN_COLD_DISTANCE_KM",
            20.0,
        ),
        min_mild_distance_km: read_positive_env_f64(
            "CAR_RANKS_TEMP_GATE_MIN_MILD_DISTANCE_KM",
            20.0,
        ),
        min_cold_charge_sessions: min_cold_charge_sessions as usize,
        min_mild_charge_sessions: min_mild_charge_sessions as usize,
        min_sensitivity_points: min_sensitivity_points as usize,
    }
}

pub(crate) fn wh_per_km_from_soc_delta(
    delta_soc_pct: f64,
    delta_km: f64,
    usable_battery_kwh: f64,
) -> Option<f64> {
    if !delta_soc_pct.is_finite()
        || !delta_km.is_finite()
        || !usable_battery_kwh.is_finite()
        || delta_soc_pct <= 0.0
        || delta_km <= 0.0
        || usable_battery_kwh <= 0.0
    {
        return None;
    }

    let energy_wh = (delta_soc_pct / 100.0) * usable_battery_kwh * 1000.0;
    let wh_per_km = energy_wh / delta_km;
    if wh_per_km.is_finite() && wh_per_km > 0.0 {
        Some(wh_per_km)
    } else {
        None
    }
}

pub(crate) fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

pub(crate) fn max_value(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
}

pub(crate) fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn linear_regression_slope(points: &[VehiclePoint]) -> Option<f64> {
    if points.len() < 2 {
        return None;
    }

    let n = points.len() as f64;
    let mean_x = points.iter().map(|p| p.temperature_c).sum::<f64>() / n;
    let mean_y = points.iter().map(|p| p.km_per_soc).sum::<f64>() / n;

    let numerator = points
        .iter()
        .map(|p| (p.temperature_c - mean_x) * (p.km_per_soc - mean_y))
        .sum::<f64>();

    let denominator = points
        .iter()
        .map(|p| (p.temperature_c - mean_x).powi(2))
        .sum::<f64>();

    if denominator <= f64::EPSILON {
        return None;
    }

    Some(numerator / denominator)
}

pub(crate) fn confidence_from_samples(sample_count: i64) -> &'static str {
    if sample_count >= 60 {
        "stable"
    } else if sample_count >= 20 {
        "medium"
    } else {
        "preview"
    }
}

pub(crate) fn timeframe_cutoff(timeframe: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let cutoff = match timeframe {
        "30d" => now - Duration::days(30),
        "90d" => now - Duration::days(90),
        "180d" => now - Duration::days(180),
        "7d" => now - Duration::days(7),
        _ => return Err(anyhow::anyhow!("unsupported timeframe: {}", timeframe)),
    };
    Ok(cutoff)
}

pub(crate) fn year_band(model_year: Option<i64>) -> String {
    match model_year {
        Some(y) => format!("{}-{}", y, y + 2),
        None => "unknown".to_string(),
    }
}

fn percentile_rank(values: &[f64], vehicle_value: f64, higher_is_better: bool) -> i64 {
    if values.is_empty() {
        return 0;
    }

    let better_or_equal = if higher_is_better {
        values.iter().filter(|v| **v <= vehicle_value).count()
    } else {
        values.iter().filter(|v| **v >= vehicle_value).count()
    };

    ((better_or_equal as f64 / values.len() as f64) * 100.0).round() as i64
}

pub(crate) fn cmp_f64_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;
    use axum::extract::State;
    use sqlx::Connection;
    use sqlx::Executor;

    #[test]
    fn temperature_bin_boundaries() {
        assert_eq!(derive_temperature_bin(-10.0), "very_cold");
        assert_eq!(derive_temperature_bin(-5.0), "very_cold");
        assert_eq!(derive_temperature_bin(-4.9), "cold");
        assert_eq!(derive_temperature_bin(5.0), "cold");
        assert_eq!(derive_temperature_bin(10.0), "cool");
        assert_eq!(derive_temperature_bin(20.0), "mild");
        assert_eq!(derive_temperature_bin(30.0), "hot");
    }

    #[test]
    fn percentile_higher_is_better() {
        let values = vec![50.0, 60.0, 70.0, 80.0];
        assert_eq!(percentile_rank(&values, 70.0, true), 75);
        assert_eq!(percentile_rank(&values, 50.0, true), 25);
    }

    #[test]
    fn percentile_lower_is_better() {
        let values = vec![10.0, 20.0, 30.0, 40.0];
        assert_eq!(percentile_rank(&values, 20.0, false), 75);
        assert_eq!(percentile_rank(&values, 40.0, false), 25);
    }

    #[test]
    fn locked_kpi_catalog_contains_core_composite_metric() {
        let spec = lookup_kpi_spec("ev_composite", "ev_composite_score");
        assert!(spec.is_some());
    }

    #[test]
    fn wh_per_km_from_soc_delta_works() {
        let wh_per_km = wh_per_km_from_soc_delta(5.0, 20.0, 60.0).expect("expected value");
        assert!((wh_per_km - 150.0).abs() < 0.0001);
    }

    #[test]
    fn score_from_kpi_map_range_fallback_uses_net_efficiency() {
        let mut kpis = BTreeMap::new();
        kpis.insert("ev_estimated_practical_range".to_string(), 280.0);
        kpis.insert("ev_net_energy_efficiency".to_string(), 160.0);

        let score = score_from_kpi_map("ev_range_efficiency", &kpis);
        assert!(score > 0.0);
        assert!(score <= 100.0);
    }

    #[test]
    fn sqlite_migration_matches_legacy_schema_snapshot() {
        assert_eq!(SQLITE_MIGRATION_0001, LEGACY_SQLITE_SCHEMA);
    }

    #[test]
    fn postgres_migration_has_expected_base_tables() {
        assert!(!POSTGRES_MIGRATION_0001.contains("PRAGMA"));
        for table_name in [
            "vehicle",
            "ingest_batch",
            "vehicle_signal_observation",
            "vehicle_diagnostic_event",
            "vehicle_session_event",
            "vehicle_charging_session",
            "vehicle_kpi_snapshot",
            "cohort_ranking_snapshot",
        ] {
            let marker = format!("CREATE TABLE IF NOT EXISTS {}", table_name);
            assert!(
                POSTGRES_MIGRATION_0001.contains(&marker),
                "missing table in postgres migration: {}",
                table_name
            );
        }
    }

    #[tokio::test]
    async fn apply_schema_records_migrations_once() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("failed to connect in-memory sqlite")?;

        apply_schema(&pool).await?;
        apply_schema(&pool).await?;

        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM schema_migration
            WHERE migration_id = '0001_init'
              AND backend = 'sqlite'
            "#,
        )
        .fetch_one(&pool)
        .await
        .context("failed to count applied migrations")?;

        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn postgres_bootstrap_migration_applies_when_env_set() -> Result<()> {
        let database_url = match std::env::var("POSTGRES_TEST_DATABASE_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(()),
        };

        let schema_name = format!("car_ranks_test_{}", Uuid::new_v4().simple());
        let mut conn = sqlx::postgres::PgConnection::connect(&database_url)
            .await
            .context("failed to connect postgres test database")?;

        conn.execute(format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name).as_str())
            .await
            .context("failed to create postgres test schema")?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .context("failed to create postgres test pool")?;
        sqlx::query(format!("SET search_path TO {}", schema_name).as_str())
            .execute(&pool)
            .await
            .context("failed to set postgres search_path")?;

        apply_postgres_schema(&pool).await?;

        let table_exists: Option<String> = sqlx::query_scalar(
            r#"
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'vehicle_kpi_snapshot'
            "#,
        )
        .fetch_optional(&pool)
        .await
        .context("failed to validate postgres migrated tables")?;
        assert_eq!(table_exists.as_deref(), Some("vehicle_kpi_snapshot"));

        sqlx::query("SET search_path TO public")
            .execute(&pool)
            .await
            .context("failed to reset pool search_path")?;
        pool.close().await;

        conn.execute("SET search_path TO public")
            .await
            .context("failed to reset search_path")?;
        conn.execute(format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name).as_str())
            .await
            .context("failed to drop postgres test schema")?;

        Ok(())
    }

    #[tokio::test]
    async fn postgres_kpi_fetch_and_charging_handler_work_when_env_set() -> Result<()> {
        let database_url = match std::env::var("POSTGRES_TEST_DATABASE_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(()),
        };

        let schema_name = format!("car_ranks_test_{}", Uuid::new_v4().simple());
        let mut admin_conn = sqlx::postgres::PgConnection::connect(&database_url)
            .await
            .context("failed to connect postgres test database")?;
        admin_conn
            .execute(format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name).as_str())
            .await
            .context("failed to create postgres test schema")?;

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .context("failed to create postgres test pool")?;
        sqlx::query(format!("SET search_path TO {}", schema_name).as_str())
            .execute(&pool)
            .await
            .context("failed to set postgres search_path")?;
        apply_postgres_schema(&pool).await?;

        let vehicle_uid = Uuid::new_v4().to_string();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO vehicle (
              vehicle_uid,
              source_account_id,
              powertrain_class,
              created_at,
              updated_at
            ) VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&vehicle_uid)
        .bind("postgres-test-account")
        .bind("bev")
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await
        .context("failed to insert postgres test vehicle")?;

        let older_ts = (now - Duration::minutes(5)).to_rfc3339();
        let newer_ts = now.to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO vehicle_kpi_snapshot (
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              computed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid)
        .bind("ev_range_efficiency")
        .bind("30d")
        .bind("ev_net_energy_efficiency")
        .bind(190.0_f64)
        .bind("wh_per_km")
        .bind("lower_is_better")
        .bind("medium")
        .bind(12_i64)
        .bind("all")
        .bind(&older_ts)
        .execute(&pool)
        .await
        .context("failed to insert older postgres range KPI")?;
        sqlx::query(
            r#"
            INSERT INTO vehicle_kpi_snapshot (
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              computed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid)
        .bind("ev_range_efficiency")
        .bind("30d")
        .bind("ev_net_energy_efficiency")
        .bind(170.0_f64)
        .bind("wh_per_km")
        .bind("lower_is_better")
        .bind("stable")
        .bind(18_i64)
        .bind("all")
        .bind(&newer_ts)
        .execute(&pool)
        .await
        .context("failed to insert newer postgres range KPI")?;
        sqlx::query(
            r#"
            INSERT INTO vehicle_kpi_snapshot (
              snapshot_id,
              vehicle_uid,
              ranking_type,
              timeframe,
              kpi_key,
              kpi_value,
              kpi_unit,
              direction,
              confidence_level,
              sample_count,
              temperature_bin,
              computed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid)
        .bind("ev_charging_performance")
        .bind("30d")
        .bind("charging_performance_score")
        .bind(82.0_f64)
        .bind("score")
        .bind("higher_is_better")
        .bind("stable")
        .bind(11_i64)
        .bind("all")
        .bind(&newer_ts)
        .execute(&pool)
        .await
        .context("failed to insert postgres charging KPI")?;

        let fetched = fetch_latest_vehicle_kpis_postgres(
            &pool,
            &vehicle_uid,
            "ev_range_efficiency",
            "30d",
            "all",
        )
        .await
        .context("failed to fetch postgres KPIs")?;
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].kpi_key, "ev_net_energy_efficiency");
        assert!((fetched[0].value - 170.0).abs() < f64::EPSILON);
        assert_eq!(fetched[0].sample_count, 18);

        let sqlite_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("failed to create sqlite state pool")?;
        apply_schema(&sqlite_pool).await?;
        let state = AppState {
            sqlite_pool,
            pg_pool: Some(pool.clone()),
            backend: DatabaseBackend::Postgres,
            signal_keys: Arc::new(load_signal_keys()?),
        };
        let query = KpiQuery {
            vehicle_uid: Uuid::parse_str(&vehicle_uid).context("invalid test vehicle uuid")?,
            timeframe: Some("30d".to_string()),
            temperature_bin: Some("all".to_string()),
            charger_type: Some("all".to_string()),
        };
        let Json(response) = get_kpis_charging(State(state), Query(query))
            .await
            .map_err(|err| {
                anyhow::anyhow!("postgres charging KPI handler failed: {}", err.message)
            })?;
        assert_eq!(response.ranking_type, "ev_charging_performance");
        assert_eq!(response.kpis.len(), 1);
        assert_eq!(response.kpis[0].kpi_key, "charging_performance_score");
        assert!((response.kpis[0].value - 82.0).abs() < f64::EPSILON);

        sqlx::query("SET search_path TO public")
            .execute(&pool)
            .await
            .context("failed to reset pool search_path")?;
        pool.close().await;

        admin_conn
            .execute(format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name).as_str())
            .await
            .context("failed to drop postgres test schema")?;

        Ok(())
    }

    #[test]
    fn temperature_sample_gate_checks() {
        let gates = TemperatureSampleGates {
            min_cold_distance_km: 20.0,
            min_mild_distance_km: 20.0,
            min_cold_charge_sessions: 1,
            min_mild_charge_sessions: 1,
            min_sensitivity_points: 6,
        };

        assert!(gates.range_gate_passed(20.0, 25.0));
        assert!(!gates.range_gate_passed(19.9, 25.0));
        assert!(!gates.range_gate_passed(20.0, 19.9));

        assert!(gates.charge_gate_passed(1, 1));
        assert!(!gates.charge_gate_passed(0, 1));
        assert!(!gates.charge_gate_passed(1, 0));
    }

    async fn test_app_state() -> Result<AppState> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("failed to connect in-memory sqlite")?;
        apply_schema(&pool).await?;
        Ok(AppState {
            sqlite_pool: pool,
            pg_pool: None,
            backend: DatabaseBackend::Sqlite,
            signal_keys: Arc::new(load_signal_keys()?),
        })
    }

    fn valid_ingest_payload(
        vehicle_uid: Uuid,
        batch_id: Uuid,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
    ) -> TelemetryBatchRequest {
        TelemetryBatchRequest {
            batch_id,
            schema_version: INGEST_SCHEMA_VERSION.to_string(),
            vehicle_uid,
            source: "OBD".to_string(),
            client: Some(ClientInfo {
                platform: Some("ios".to_string()),
                app_version: Some("1.0.0-test".to_string()),
                adapter_fingerprint: Some("adapter-test".to_string()),
            }),
            capture_window: CaptureWindow {
                started_at,
                ended_at,
                sample_interval_seconds: Some(60),
            },
            records: vec![TelemetryRecord {
                observed_at: started_at + Duration::seconds(5),
                signal_key: "speed.vehicle".to_string(),
                value_number: Some(42.0),
                value_string: None,
                value_bool: None,
                value_json: None,
                unit: Some("km/h".to_string()),
                status: "ok".to_string(),
                confidence: Some(0.95),
                source_signal: Some("01_0D".to_string()),
                freshness_ttl_seconds: Some(30),
                temperature_bin: None,
                is_temperature_estimated: Some(false),
                session_id: None,
                raw_payload_ref: None,
            }],
            session_events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    #[tokio::test]
    async fn ingest_duplicate_same_envelope_returns_duplicate_true() -> Result<()> {
        let state = test_app_state().await?;
        let now = Utc::now();
        let vehicle_uid = Uuid::new_v4();
        let batch_id = Uuid::new_v4();
        let payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
        );

        let Json(first_response) = post_telemetry_batches(State(state.clone()), Json(payload))
            .await
            .map_err(|err| anyhow::anyhow!("first ingest failed: {} {}", err.error, err.message))?;
        assert!(first_response.accepted);
        assert!(!first_response.duplicate);

        let duplicate_payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
        );
        let Json(duplicate_response) =
            post_telemetry_batches(State(state.clone()), Json(duplicate_payload))
                .await
                .map_err(|err| {
                    anyhow::anyhow!("duplicate ingest failed: {} {}", err.error, err.message)
                })?;
        assert!(duplicate_response.accepted);
        assert!(duplicate_response.duplicate);
        assert_eq!(duplicate_response.records_accepted, 0);
        Ok(())
    }

    #[tokio::test]
    async fn ingest_duplicate_with_different_envelope_returns_conflict() -> Result<()> {
        let state = test_app_state().await?;
        let now = Utc::now();
        let vehicle_uid = Uuid::new_v4();
        let batch_id = Uuid::new_v4();
        let payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
        );
        let _ = post_telemetry_batches(State(state.clone()), Json(payload))
            .await
            .map_err(|err| anyhow::anyhow!("first ingest failed: {} {}", err.error, err.message))?;

        let conflict_payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::seconds(10),
        );
        let err = post_telemetry_batches(State(state.clone()), Json(conflict_payload))
            .await
            .expect_err("expected idempotency conflict");

        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(err.error, "conflict");
        Ok(())
    }

    #[tokio::test]
    async fn ingest_rejects_unsupported_schema_version() -> Result<()> {
        let state = test_app_state().await?;
        let now = Utc::now();
        let vehicle_uid = Uuid::new_v4();
        let batch_id = Uuid::new_v4();
        let mut payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
        );
        payload.schema_version = "1.0".to_string();

        let err = post_telemetry_batches(State(state.clone()), Json(payload))
            .await
            .expect_err("expected schema_version rejection");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.error, "bad_request");
        Ok(())
    }

    #[tokio::test]
    async fn ingest_rejects_record_outside_capture_window() -> Result<()> {
        let state = test_app_state().await?;
        let now = Utc::now();
        let vehicle_uid = Uuid::new_v4();
        let batch_id = Uuid::new_v4();
        let mut payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
        );
        payload.records[0].observed_at = now;

        let err = post_telemetry_batches(State(state.clone()), Json(payload))
            .await
            .expect_err("expected out-of-window rejection");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.error, "bad_request");
        Ok(())
    }

    #[tokio::test]
    async fn ingest_rejects_unknown_session_event_type() -> Result<()> {
        let state = test_app_state().await?;
        let now = Utc::now();
        let vehicle_uid = Uuid::new_v4();
        let batch_id = Uuid::new_v4();
        let mut payload = valid_ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
        );
        payload.session_events.push(SessionEventInput {
            event_type: "invalid_session_event".to_string(),
            observed_at: now - Duration::minutes(1) + Duration::seconds(10),
            session_id: Uuid::new_v4(),
        });

        let err = post_telemetry_batches(State(state.clone()), Json(payload))
            .await
            .expect_err("expected session event type rejection");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.error, "bad_request");
        Ok(())
    }

    #[tokio::test]
    async fn temperature_rankings_skip_vehicle_when_range_gate_fails() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("failed to connect in-memory sqlite")?;
        apply_schema(&pool).await?;

        let vehicle_uid = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO vehicle (
              vehicle_uid,
              source_account_id,
              powertrain_class,
              created_at,
              updated_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&vehicle_uid)
        .bind("test-account")
        .bind("bev")
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await
        .context("failed to insert vehicle")?;

        for i in 0..6 {
            let ts = now - Duration::hours(4) + Duration::minutes(i * 5);
            let odo_km = 1000.0 + i as f64;
            let soc_pct = 90.0 - (i as f64 * 0.5);
            let temp_c = if i < 3 { 20.0 } else { 0.0 };

            for (signal_key, value, temp_bin) in [
                ("distance.odometer", odo_km, None),
                ("ev.soc_pct", soc_pct, None),
                (
                    "environment.ambient_temp_c",
                    temp_c,
                    Some(derive_temperature_bin(temp_c).to_string()),
                ),
            ] {
                sqlx::query(
                    r#"
                    INSERT INTO vehicle_signal_observation (
                      observation_id,
                      vehicle_uid,
                      signal_key,
                      value_number,
                      observed_at,
                      ingested_at,
                      source,
                      status,
                      temperature_bin,
                      is_temperature_estimated
                    ) VALUES (?, ?, ?, ?, ?, ?, 'OBD', 'ok', ?, 0)
                    "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(&vehicle_uid)
                .bind(signal_key)
                .bind(value)
                .bind(ts.to_rfc3339())
                .bind(now.to_rfc3339())
                .bind(temp_bin)
                .execute(&pool)
                .await
                .with_context(|| format!("failed to insert observation {}", signal_key))?;
            }
        }

        for (session_id, avg_power_kw, temp_bin) in [
            (Uuid::new_v4().to_string(), 62.0, "mild"),
            (Uuid::new_v4().to_string(), 34.0, "cold"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO vehicle_charging_session (
                  charging_session_id,
                  vehicle_uid,
                  session_id,
                  started_at,
                  ended_at,
                  status,
                  charger_type,
                  avg_charge_power_kw,
                  temperature_bin,
                  temperature_is_estimated,
                  sample_count,
                  created_at,
                  updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&vehicle_uid)
            .bind(session_id)
            .bind((now - Duration::hours(2)).to_rfc3339())
            .bind((now - Duration::hours(1)).to_rfc3339())
            .bind("complete")
            .bind("dc")
            .bind(avg_power_kw)
            .bind(temp_bin)
            .bind(0_i64)
            .bind(2_i64)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&pool)
            .await
            .context("failed to insert charging session")?;
        }

        let _ = recompute_temperature_kpis(&pool).await?;
        let _ = rebuild_temperature_rankings(&pool).await?;

        let temp_keys: HashSet<String> = sqlx::query(
            r#"
            SELECT DISTINCT kpi_key
            FROM vehicle_kpi_snapshot
            WHERE vehicle_uid = ?
              AND ranking_type = 'ev_temperature_impact'
              AND timeframe = '30d'
              AND temperature_bin = 'cold'
            "#,
        )
        .bind(&vehicle_uid)
        .fetch_all(&pool)
        .await
        .context("failed to fetch temperature KPI keys")?
        .into_iter()
        .map(|row| row.try_get::<String, _>("kpi_key"))
        .collect::<std::result::Result<HashSet<_>, _>>()
        .context("failed to parse temperature KPI keys")?;

        assert!(!temp_keys.contains("cold_weather_range_retention"));
        assert!(!temp_keys.contains("range_temperature_sensitivity_index"));
        assert!(temp_keys.contains("cold_weather_charge_speed_retention"));

        let ranking_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM cohort_ranking_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = '30d'
            "#,
        )
        .fetch_one(&pool)
        .await
        .context("failed to count temperature rankings")?;

        assert_eq!(ranking_count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn end_to_end_kpi_job_materializes_locked_kpi_sets() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("failed to connect in-memory sqlite")?;
        apply_schema(&pool).await?;

        let state = AppState {
            sqlite_pool: pool.clone(),
            pg_pool: None,
            backend: DatabaseBackend::Sqlite,
            signal_keys: Arc::new(load_signal_keys()?),
        };

        let vehicle_uid = Uuid::new_v4();
        let mild_charge_session_id = Uuid::new_v4();
        let cold_charge_session_id = Uuid::new_v4();
        let now = Utc::now();

        let drive_start = now - Duration::hours(6);
        let mild_charge_start = now - Duration::hours(3);
        let mild_charge_stop = now - Duration::hours(2) - Duration::minutes(30);
        let cold_charge_start = now - Duration::hours(2);
        let cold_charge_stop = now - Duration::hours(1) - Duration::minutes(20);

        let number_record = |observed_at: DateTime<Utc>,
                             signal_key: &str,
                             value_number: f64,
                             unit: Option<&str>,
                             session_id: Option<Uuid>|
         -> TelemetryRecord {
            TelemetryRecord {
                observed_at,
                signal_key: signal_key.to_string(),
                value_number: Some(value_number),
                value_string: None,
                value_bool: None,
                value_json: None,
                unit: unit.map(str::to_string),
                status: "ok".to_string(),
                confidence: Some(1.0),
                source_signal: Some(signal_key.to_string()),
                freshness_ttl_seconds: Some(30),
                temperature_bin: if signal_key == "environment.ambient_temp_c" {
                    Some(derive_temperature_bin(value_number).to_string())
                } else {
                    None
                },
                is_temperature_estimated: Some(false),
                session_id,
                raw_payload_ref: None,
            }
        };

        let string_record = |observed_at: DateTime<Utc>,
                             signal_key: &str,
                             value_string: &str,
                             session_id: Option<Uuid>|
         -> TelemetryRecord {
            TelemetryRecord {
                observed_at,
                signal_key: signal_key.to_string(),
                value_number: None,
                value_string: Some(value_string.to_string()),
                value_bool: None,
                value_json: None,
                unit: None,
                status: "ok".to_string(),
                confidence: Some(1.0),
                source_signal: Some(signal_key.to_string()),
                freshness_ttl_seconds: Some(60),
                temperature_bin: None,
                is_temperature_estimated: Some(false),
                session_id,
                raw_payload_ref: None,
            }
        };

        let mut records = Vec::new();
        for i in 0..21 {
            let ts = drive_start + Duration::minutes(i * 5);
            let odo_km = 1000.0 + (i as f64 * 2.5);
            let soc_pct = 90.0 - (i as f64 * 0.35);
            let ambient_temp_c = if i < 11 { 20.0 } else { 0.0 };
            let speed_kmh = if i % 2 == 0 { 35.0 } else { 95.0 };
            let regen_kw = 4.0 + (i % 2) as f64;
            let traction_kw = 18.0 + (i % 3) as f64;

            records.push(number_record(
                ts,
                "distance.odometer",
                odo_km,
                Some("km"),
                None,
            ));
            records.push(number_record(ts, "ev.soc_pct", soc_pct, Some("%"), None));
            records.push(number_record(
                ts,
                "environment.ambient_temp_c",
                ambient_temp_c,
                Some("C"),
                None,
            ));
            records.push(number_record(
                ts,
                "speed.vehicle",
                speed_kmh,
                Some("km/h"),
                None,
            ));
            records.push(number_record(
                ts,
                "ev.regen_power_kw",
                regen_kw,
                Some("kW"),
                None,
            ));
            records.push(number_record(
                ts,
                "ev.traction_power_kw",
                traction_kw,
                Some("kW"),
                None,
            ));
        }

        for (session_id, start, stop, power_a, power_b, temp_a, temp_b, soc_a, soc_b) in [
            (
                mild_charge_session_id,
                mild_charge_start,
                mild_charge_stop,
                62.0,
                58.0,
                20.0,
                21.0,
                40.0,
                50.0,
            ),
            (
                cold_charge_session_id,
                cold_charge_start,
                cold_charge_stop,
                36.0,
                34.0,
                0.0,
                1.0,
                52.0,
                60.0,
            ),
        ] {
            let mid = start + Duration::minutes(10);
            let near_end = stop - Duration::minutes(5);

            records.push(number_record(
                start,
                "ev.soc_pct",
                soc_a,
                Some("%"),
                Some(session_id),
            ));
            records.push(number_record(
                near_end,
                "ev.soc_pct",
                soc_b,
                Some("%"),
                Some(session_id),
            ));
            records.push(number_record(
                start,
                "ev.charge_power_kw",
                power_a,
                Some("kW"),
                Some(session_id),
            ));
            records.push(number_record(
                mid,
                "ev.charge_power_kw",
                power_b,
                Some("kW"),
                Some(session_id),
            ));
            records.push(number_record(
                start,
                "environment.ambient_temp_c",
                temp_a,
                Some("C"),
                Some(session_id),
            ));
            records.push(number_record(
                near_end,
                "environment.ambient_temp_c",
                temp_b,
                Some("C"),
                Some(session_id),
            ));
            records.push(number_record(
                mid,
                "ev.battery_temp_c",
                if temp_a > 10.0 { 25.0 } else { 6.0 },
                Some("C"),
                Some(session_id),
            ));
            records.push(string_record(
                mid,
                "ev.charger_type",
                "dc_fast",
                Some(session_id),
            ));
            records.push(string_record(
                start,
                "ev.charging_state",
                "charging",
                Some(session_id),
            ));
        }

        let payload = TelemetryBatchRequest {
            batch_id: Uuid::new_v4(),
            schema_version: "0.2".to_string(),
            vehicle_uid,
            source: "OBD".to_string(),
            client: Some(ClientInfo {
                platform: Some("ios".to_string()),
                app_version: Some("1.0.0-test".to_string()),
                adapter_fingerprint: Some("adapter-test-123".to_string()),
            }),
            capture_window: CaptureWindow {
                started_at: drive_start - Duration::minutes(1),
                ended_at: now,
                sample_interval_seconds: Some(60),
            },
            records,
            session_events: vec![
                SessionEventInput {
                    event_type: "charging_session_start".to_string(),
                    observed_at: mild_charge_start,
                    session_id: mild_charge_session_id,
                },
                SessionEventInput {
                    event_type: "charging_session_stop".to_string(),
                    observed_at: mild_charge_stop,
                    session_id: mild_charge_session_id,
                },
                SessionEventInput {
                    event_type: "charging_session_start".to_string(),
                    observed_at: cold_charge_start,
                    session_id: cold_charge_session_id,
                },
                SessionEventInput {
                    event_type: "charging_session_stop".to_string(),
                    observed_at: cold_charge_stop,
                    session_id: cold_charge_session_id,
                },
            ],
            diagnostics: vec![DiagnosticInput {
                observed_at: now - Duration::minutes(45),
                mil_on: Some(true),
                dtcs_active: Some(vec!["P0ABC".to_string(), "P0DEF".to_string()]),
            }],
        };

        let Json(ingest_response) = post_telemetry_batches(State(state.clone()), Json(payload))
            .await
            .map_err(|err| anyhow::anyhow!("ingest failed: {} {}", err.error, err.message))?;
        assert!(ingest_response.accepted);
        assert!(!ingest_response.duplicate);
        assert_eq!(ingest_response.records_rejected, 0);

        let job = run_kpi_job(&pool)
            .await
            .map_err(|err| anyhow::anyhow!("kpi job failed: {} {}", err.error, err.message))?;
        assert!(job.ok);
        assert_eq!(job.recomputed_vehicles, 1);
        assert!(job.kpi_rows_upserted > 0);
        assert!(job.ranking_rows_upserted > 0);

        let vehicle_uid_text = vehicle_uid.to_string();
        let expected_by_ranking: [(&str, &[&str]); 4] = [
            (
                "ev_range_efficiency",
                &[
                    "ev_net_energy_efficiency",
                    "ev_estimated_practical_range",
                    "ev_urban_efficiency",
                    "ev_highway_efficiency",
                    "regeneration_recovery_ratio",
                    "soc_depletion_rate_per_100km",
                    "ev_range_efficiency_score",
                ],
            ),
            (
                "ev_charging_performance",
                &[
                    "temp_adjusted_charge_acceptance_score",
                    "cold_weather_charge_speed_retention",
                    "charging_performance_score",
                ],
            ),
            (
                "ev_temperature_impact",
                &[
                    "cold_weather_range_retention",
                    "range_temperature_sensitivity_index",
                    "cold_weather_charge_speed_retention",
                ],
            ),
            (
                "ev_composite",
                &[
                    "ev_composite_base_score",
                    "ev_health_modifier_penalty",
                    "ev_composite_score",
                ],
            ),
        ];

        for (ranking_type, expected_keys) in expected_by_ranking {
            let rows = sqlx::query(
                r#"
                SELECT DISTINCT kpi_key
                FROM vehicle_kpi_snapshot
                WHERE vehicle_uid = ?
                  AND ranking_type = ?
                  AND timeframe = '30d'
                "#,
            )
            .bind(&vehicle_uid_text)
            .bind(ranking_type)
            .fetch_all(&pool)
            .await
            .with_context(|| format!("failed to fetch keys for {}", ranking_type))?;

            let keys: HashSet<String> = rows
                .into_iter()
                .map(|row| row.try_get::<String, _>("kpi_key"))
                .collect::<std::result::Result<HashSet<_>, _>>()
                .with_context(|| format!("failed to parse keys for {}", ranking_type))?;

            for expected_key in expected_keys {
                assert!(
                    keys.contains(*expected_key),
                    "missing {} for ranking_type {}",
                    expected_key,
                    ranking_type
                );
            }
        }

        let composite_rows = sqlx::query(
            r#"
            SELECT kpi_key, kpi_value
            FROM vehicle_kpi_snapshot
            WHERE vehicle_uid = ?
              AND ranking_type = 'ev_composite'
              AND timeframe = '30d'
            "#,
        )
        .bind(&vehicle_uid_text)
        .fetch_all(&pool)
        .await
        .context("failed to fetch composite KPI values")?;

        let mut composite_map = BTreeMap::new();
        for row in composite_rows {
            let key: String = row.try_get("kpi_key")?;
            let value: f64 = row.try_get("kpi_value")?;
            composite_map.insert(key, value);
        }

        let base = *composite_map
            .get("ev_composite_base_score")
            .context("missing ev_composite_base_score")?;
        let penalty = *composite_map
            .get("ev_health_modifier_penalty")
            .context("missing ev_health_modifier_penalty")?;
        let final_score = *composite_map
            .get("ev_composite_score")
            .context("missing ev_composite_score")?;

        assert!(penalty > 0.0);
        assert!(final_score <= base);
        assert!((final_score - (base - penalty).clamp(0.0, 100.0)).abs() < 0.0001);

        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
            "ev_temperature_impact",
        ] {
            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*)
                FROM cohort_ranking_snapshot
                WHERE ranking_type = ?
                  AND timeframe = '30d'
                "#,
            )
            .bind(ranking_type)
            .fetch_one(&pool)
            .await
            .with_context(|| format!("failed to count rankings for {}", ranking_type))?;

            assert!(count > 0, "expected ranking rows for {}", ranking_type);
        }

        Ok(())
    }
}
