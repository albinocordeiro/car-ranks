use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    signal_keys: Arc<HashSet<String>>,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
    message: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    error: String,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "bad_request".to_string(),
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: "not_found".to_string(),
            message: message.into(),
        }
    }

    fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error: "unprocessable_entity".to_string(),
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "internal_error".to_string(),
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self::internal(format!("{}", value))
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorBody {
            error: self.error,
            message: self.message,
        });
        (self.status, body).into_response()
    }
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
    kpi_key: String,
    value: f64,
    unit: String,
    direction: String,
    confidence_level: String,
    sample_count: i64,
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
    key: &'static str,
    value: f64,
    unit: &'static str,
    direction: &'static str,
    sample_count: i64,
    confidence_level: &'static str,
}

#[derive(Debug)]
struct VehicleRankingSeed {
    vehicle_uid: String,
    make: String,
    model: String,
    trim: String,
    model_year: Option<i64>,
    range_retention: Option<f64>,
    sensitivity: Option<f64>,
    charge_retention: Option<f64>,
    confidence_level: String,
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

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://car_ranks.db".to_string());
    let connect_options = SqliteConnectOptions::from_str(&database_url)
        .context("invalid DATABASE_URL")?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(connect_options)
        .await
        .context("failed to connect sqlite")?;

    apply_schema(&pool).await?;

    let app_state = AppState { pool, signal_keys };

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
    if payload.source.to_uppercase() != "OBD" {
        return Err(ApiError::bad_request("source must be OBD for MVP"));
    }

    if payload.capture_window.ended_at <= payload.capture_window.started_at {
        return Err(ApiError::bad_request(
            "capture_window.ended_at must be after capture_window.started_at",
        ));
    }

    if payload.records.len() > 5_000 {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            error: "payload_too_large".to_string(),
            message: "maximum records per batch is 5000".to_string(),
        });
    }

    if payload.records.is_empty()
        && payload.session_events.is_empty()
        && payload.diagnostics.is_empty()
    {
        return Err(ApiError::bad_request(
            "records can only be empty when session_events or diagnostics are present",
        ));
    }

    if let Some(client) = &payload.client {
        if let Some(platform) = &client.platform {
            if platform.to_lowercase() != "ios" {
                return Err(ApiError::bad_request("client.platform must be ios for MVP"));
            }
        }
    }

    let source_account_id = payload
        .client
        .as_ref()
        .and_then(|c| c.adapter_fingerprint.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let _client_app_version = payload.client.as_ref().and_then(|c| c.app_version.clone());

    let mut tx = state
        .pool
        .begin()
        .await
        .context("failed to open transaction")?;

    let duplicate = sqlx::query("SELECT 1 FROM ingest_batch WHERE batch_id = ? LIMIT 1")
        .bind(payload.batch_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .context("failed to check idempotency")?
        .is_some();

    let ingest_id = Uuid::new_v4();

    if duplicate {
        tx.commit().await.context("failed to commit duplicate tx")?;
        return Ok(Json(IngestResponse {
            accepted: true,
            batch_id: payload.batch_id,
            ingest_id,
            duplicate: true,
            records_received: payload.records.len(),
            records_accepted: 0,
            records_rejected: 0,
            errors: Vec::new(),
            next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
        }));
    }

    let now = now_str();
    let vehicle_uid_str = payload.vehicle_uid.to_string();

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO vehicle (
            vehicle_uid,
            source_account_id,
            powertrain_class,
            created_at,
            updated_at
        ) VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&vehicle_uid_str)
    .bind(&source_account_id)
    .bind("bev")
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .context("failed to ensure vehicle")?;

    sqlx::query(
        r#"
        INSERT INTO ingest_batch (
            batch_id,
            vehicle_uid,
            schema_version,
            source,
            capture_started_at,
            capture_ended_at,
            received_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(payload.batch_id.to_string())
    .bind(&vehicle_uid_str)
    .bind(&payload.schema_version)
    .bind(&payload.source)
    .bind(payload.capture_window.started_at.to_rfc3339())
    .bind(payload.capture_window.ended_at.to_rfc3339())
    .bind(&now)
    .execute(&mut *tx)
    .await
    .context("failed to insert ingest batch")?;

    let mut errors = Vec::new();
    let mut accepted = 0usize;

    for (index, record) in payload.records.iter().enumerate() {
        if !state.signal_keys.contains(&record.signal_key) {
            errors.push(IngestRecordError {
                record_index: index,
                code: "unknown_signal_key".to_string(),
                message: "signal_key not present in active v0.2 registry".to_string(),
            });
            continue;
        }

        if !(record.status == "ok"
            || record.status == "stale"
            || record.status == "unavailable"
            || record.status == "not_supported"
            || record.status == "permission_denied"
            || record.status == "error")
        {
            errors.push(IngestRecordError {
                record_index: index,
                code: "invalid_status".to_string(),
                message: "invalid status enum".to_string(),
            });
            continue;
        }

        if let Some(confidence) = record.confidence {
            if !(0.0..=1.0).contains(&confidence) {
                errors.push(IngestRecordError {
                    record_index: index,
                    code: "invalid_confidence".to_string(),
                    message: "confidence must be between 0 and 1".to_string(),
                });
                continue;
            }
        }

        let derived_temperature_bin = record.temperature_bin.clone().or_else(|| {
            match (record.signal_key.as_str(), record.value_number) {
                ("environment.ambient_temp_c", Some(temp)) => {
                    Some(derive_temperature_bin(temp).to_string())
                }
                _ => None,
            }
        });

        let session_id = record.session_id.map(|id| id.to_string());
        let value_json_text = record.value_json.as_ref().map(|v| v.to_string());

        sqlx::query(
            r#"
            INSERT INTO vehicle_signal_observation (
                observation_id,
                vehicle_uid,
                batch_id,
                session_id,
                signal_key,
                value_number,
                value_string,
                value_bool,
                value_json,
                unit,
                observed_at,
                ingested_at,
                source,
                source_signal,
                status,
                confidence,
                freshness_ttl_seconds,
                temperature_bin,
                is_temperature_estimated,
                raw_payload_ref
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_str)
        .bind(payload.batch_id.to_string())
        .bind(session_id)
        .bind(&record.signal_key)
        .bind(record.value_number)
        .bind(&record.value_string)
        .bind(record.value_bool.map(i64::from))
        .bind(value_json_text)
        .bind(&record.unit)
        .bind(record.observed_at.to_rfc3339())
        .bind(&now)
        .bind("OBD")
        .bind(&record.source_signal)
        .bind(&record.status)
        .bind(record.confidence)
        .bind(record.freshness_ttl_seconds)
        .bind(derived_temperature_bin)
        .bind(record.is_temperature_estimated.unwrap_or(false) as i64)
        .bind(&record.raw_payload_ref)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to insert observation at index {}", index))?;

        accepted += 1;
    }

    for event in &payload.session_events {
        if let Some((session_type, event_type)) = map_session_event(&event.event_type) {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO vehicle_session_event (
                    session_event_id,
                    vehicle_uid,
                    session_id,
                    session_type,
                    event_type,
                    observed_at,
                    ingested_at,
                    source,
                    raw_payload_ref
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&vehicle_uid_str)
            .bind(event.session_id.to_string())
            .bind(session_type)
            .bind(event_type)
            .bind(event.observed_at.to_rfc3339())
            .bind(&now)
            .bind("OBD")
            .bind(None::<String>)
            .execute(&mut *tx)
            .await
            .context("failed to insert session event")?;
        }
    }

    for diag in &payload.diagnostics {
        if let Some(mil_on) = diag.mil_on {
            sqlx::query(
                r#"
                INSERT INTO vehicle_diagnostic_event (
                    event_id,
                    vehicle_uid,
                    batch_id,
                    event_type,
                    observed_at,
                    ingested_at,
                    source
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&vehicle_uid_str)
            .bind(payload.batch_id.to_string())
            .bind(if mil_on { "MIL_ON" } else { "MIL_OFF" })
            .bind(diag.observed_at.to_rfc3339())
            .bind(&now)
            .bind("OBD")
            .execute(&mut *tx)
            .await
            .context("failed to insert MIL diagnostic event")?;
        }

        if let Some(dtcs) = &diag.dtcs_active {
            for code in dtcs {
                sqlx::query(
                    r#"
                    INSERT INTO vehicle_diagnostic_event (
                        event_id,
                        vehicle_uid,
                        batch_id,
                        event_type,
                        code,
                        observed_at,
                        ingested_at,
                        source
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(&vehicle_uid_str)
                .bind(payload.batch_id.to_string())
                .bind("DTC_ACTIVE")
                .bind(code)
                .bind(diag.observed_at.to_rfc3339())
                .bind(&now)
                .bind("OBD")
                .execute(&mut *tx)
                .await
                .context("failed to insert DTC diagnostic event")?;
            }
        }
    }

    tx.commit().await.context("failed to commit ingest tx")?;

    Ok(Json(IngestResponse {
        accepted: true,
        batch_id: payload.batch_id,
        ingest_id,
        duplicate: false,
        records_received: payload.records.len(),
        records_accepted: accepted,
        records_rejected: payload.records.len().saturating_sub(accepted),
        errors,
        next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
    }))
}

async fn post_recompute_kpis(State(state): State<AppState>) -> Result<Json<JobResponse>, ApiError> {
    run_kpi_job(&state.pool).await.map(Json)
}

async fn post_build_rankings(State(state): State<AppState>) -> Result<Json<JobResponse>, ApiError> {
    run_kpi_job(&state.pool).await.map(Json)
}

async fn get_kpis_me(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params.temperature_bin.unwrap_or_else(|| "all".to_string());
    let vehicle_uid = params.vehicle_uid.to_string();

    if temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filter is only supported for /v1/kpis/temperature-impact in this thin slice",
        ));
    }

    let kpis = fetch_latest_vehicle_kpis(
        &state.pool,
        &vehicle_uid,
        "ev_range_efficiency",
        &timeframe,
        &temperature_bin,
    )
    .await?;

    if kpis.is_empty() {
        return Err(ApiError::not_found(
            "range/efficiency KPIs are not available for this vehicle",
        ));
    }

    Ok(Json(GenericKpiResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        timeframe,
        temperature_bin,
        ranking_type: "ev_range_efficiency".to_string(),
        kpis,
    }))
}

async fn get_kpis_charging(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params.temperature_bin.unwrap_or_else(|| "all".to_string());
    let _charger_type = params.charger_type.unwrap_or_else(|| "all".to_string());
    let vehicle_uid = params.vehicle_uid.to_string();

    if temperature_bin != "all" && temperature_bin != "cold" {
        return Err(ApiError::unprocessable(
            "unsupported temperature_bin for charging KPIs in thin slice",
        ));
    }

    let kpis = fetch_latest_vehicle_kpis(
        &state.pool,
        &vehicle_uid,
        "ev_charging_performance",
        &timeframe,
        &temperature_bin,
    )
    .await?;

    if kpis.is_empty() {
        return Err(ApiError::not_found(
            "charging KPIs are not available for this vehicle",
        ));
    }

    Ok(Json(GenericKpiResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        timeframe,
        temperature_bin,
        ranking_type: "ev_charging_performance".to_string(),
        kpis,
    }))
}

async fn get_kpis_temperature_impact(
    State(state): State<AppState>,
    Query(params): Query<KpiTempQuery>,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    let timeframe = params.timeframe.unwrap_or_else(|| "90d".to_string());
    let baseline_bin = params
        .baseline_temperature_bin
        .unwrap_or_else(|| "mild".to_string());
    let compare_bin = params
        .compare_temperature_bin
        .unwrap_or_else(|| "cold".to_string());
    let temperature_bin = "cold";

    let vehicle_uid = params.vehicle_uid.to_string();

    let rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value, kpi_unit, direction, confidence_level, sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = ?
          AND ranking_type = 'ev_temperature_impact'
          AND timeframe = ?
          AND temperature_bin = ?
          AND baseline_temperature_bin = ?
          AND compare_temperature_bin = ?
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.baseline_temperature_bin = ks.baseline_temperature_bin
                AND ks2.compare_temperature_bin = ks.compare_temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(&vehicle_uid)
    .bind(&timeframe)
    .bind(temperature_bin)
    .bind(&baseline_bin)
    .bind(&compare_bin)
    .fetch_all(&state.pool)
    .await
    .context("failed to fetch KPI rows")?;

    if rows.is_empty() {
        return Err(ApiError::not_found(
            "temperature impact metrics are not available for this vehicle",
        ));
    }

    let vehicle_row = sqlx::query("SELECT make, model FROM vehicle WHERE vehicle_uid = ?")
        .bind(&vehicle_uid)
        .fetch_one(&state.pool)
        .await
        .context("failed to fetch vehicle metadata")?;

    let make = vehicle_row
        .try_get::<Option<String>, _>("make")
        .context("failed to parse vehicle.make")?
        .unwrap_or_else(|| "unknown".to_string());
    let model = vehicle_row
        .try_get::<Option<String>, _>("model")
        .context("failed to parse vehicle.model")?
        .unwrap_or_else(|| "unknown".to_string());

    let mut metrics = Vec::new();
    let mut percentiles = BTreeMap::new();
    let mut cohort_size = 0usize;

    for row in rows {
        let kpi_key: String = row.try_get("kpi_key").context("failed to parse kpi_key")?;
        let value: f64 = row
            .try_get("kpi_value")
            .context("failed to parse kpi_value")?;
        let unit = row
            .try_get::<Option<String>, _>("kpi_unit")
            .context("failed to parse kpi_unit")?
            .unwrap_or_else(|| "score".to_string());
        let direction: String = row
            .try_get("direction")
            .context("failed to parse direction")?;
        let confidence_level: String = row
            .try_get("confidence_level")
            .context("failed to parse confidence_level")?;
        let sample_count: i64 = row
            .try_get("sample_count")
            .context("failed to parse sample_count")?;

        metrics.push(KpiMetric {
            kpi_key: kpi_key.clone(),
            value,
            unit,
            direction: direction.clone(),
            confidence_level,
            sample_count,
        });

        let cohort_values = sqlx::query(
            r#"
            SELECT ks.kpi_value
            FROM vehicle_kpi_snapshot ks
            JOIN vehicle v ON v.vehicle_uid = ks.vehicle_uid
            WHERE ks.kpi_key = ?
              AND ks.ranking_type = 'ev_temperature_impact'
              AND ks.timeframe = ?
              AND ks.temperature_bin = ?
              AND ks.baseline_temperature_bin = ?
              AND ks.compare_temperature_bin = ?
              AND ks.computed_at = (
                  SELECT MAX(ks2.computed_at)
                  FROM vehicle_kpi_snapshot ks2
                  WHERE ks2.vehicle_uid = ks.vehicle_uid
                    AND ks2.ranking_type = ks.ranking_type
                    AND ks2.timeframe = ks.timeframe
                    AND ks2.temperature_bin = ks.temperature_bin
                    AND ks2.baseline_temperature_bin = ks.baseline_temperature_bin
                    AND ks2.compare_temperature_bin = ks.compare_temperature_bin
                    AND ks2.kpi_key = ks.kpi_key
              )
              AND COALESCE(v.make, 'unknown') = ?
              AND COALESCE(v.model, 'unknown') = ?
            "#,
        )
        .bind(&kpi_key)
        .bind(&timeframe)
        .bind(temperature_bin)
        .bind(&baseline_bin)
        .bind(&compare_bin)
        .bind(&make)
        .bind(&model)
        .fetch_all(&state.pool)
        .await
        .context("failed to fetch cohort values for percentile")?;

        let values: Vec<f64> = cohort_values
            .into_iter()
            .filter_map(|r| r.try_get::<Option<f64>, _>("kpi_value").ok().flatten())
            .collect();

        cohort_size = cohort_size.max(values.len());
        let pct = percentile_rank(&values, value, direction == "higher_is_better");
        percentiles.insert(kpi_key, pct);
    }

    Ok(Json(TemperatureImpactResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        baseline_temperature_bin: baseline_bin,
        compare_temperature_bin: compare_bin,
        metrics,
        cohort_benchmark: CohortBenchmark {
            cohort_size,
            percentiles,
        },
    }))
}

async fn fetch_latest_vehicle_kpis(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>> {
    let rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value, kpi_unit, direction, confidence_level, sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = ?
          AND ranking_type = ?
          AND timeframe = ?
          AND temperature_bin = ?
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch latest vehicle KPIs")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(KpiMetric {
            kpi_key: row
                .try_get("kpi_key")
                .context("failed to parse kpi_key in fetch_latest_vehicle_kpis")?,
            value: row
                .try_get("kpi_value")
                .context("failed to parse kpi_value in fetch_latest_vehicle_kpis")?,
            unit: row
                .try_get::<Option<String>, _>("kpi_unit")
                .context("failed to parse kpi_unit in fetch_latest_vehicle_kpis")?
                .unwrap_or_else(|| "score".to_string()),
            direction: row
                .try_get("direction")
                .context("failed to parse direction in fetch_latest_vehicle_kpis")?,
            confidence_level: row
                .try_get("confidence_level")
                .context("failed to parse confidence_level in fetch_latest_vehicle_kpis")?,
            sample_count: row
                .try_get("sample_count")
                .context("failed to parse sample_count in fetch_latest_vehicle_kpis")?,
        });
    }

    Ok(out)
}

async fn get_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    let supported_ranking_type = matches!(
        params.ranking_type.as_str(),
        "ev_temperature_impact"
            | "ev_range_efficiency"
            | "ev_charging_performance"
            | "ev_composite"
    );

    if !supported_ranking_type {
        return Err(ApiError::unprocessable("unsupported ranking_type"));
    }

    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params.temperature_bin.unwrap_or_else(|| "all".to_string());
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    if params.ranking_type != "ev_temperature_impact" && temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filters are currently supported only for ev_temperature_impact",
        ));
    }

    let latest_computed = sqlx::query(
        r#"
        SELECT MAX(computed_at) AS computed_at
        FROM cohort_ranking_snapshot
        WHERE ranking_type = ?
          AND timeframe = ?
          AND temperature_bin = ?
        "#,
    )
    .bind(&params.ranking_type)
    .bind(&timeframe)
    .bind(&temperature_bin)
    .fetch_one(&state.pool)
    .await
    .context("failed to query ranking snapshot timestamp")?
    .try_get::<Option<String>, _>("computed_at")
    .context("failed to parse ranking computed_at")?;

    let computed_at = latest_computed
        .ok_or_else(|| ApiError::not_found("no ranking snapshot available for this filter"))?;

    let mut sql = String::from(
        r#"
        SELECT
          r.rank_position,
          r.vehicle_uid,
          r.score,
          r.confidence_level,
          r.cohort_key,
          r.cohort_size,
          r.sample_gate_passed
        FROM cohort_ranking_snapshot r
        JOIN vehicle v ON v.vehicle_uid = r.vehicle_uid
        WHERE r.ranking_type = ?
          AND r.timeframe = ?
          AND r.temperature_bin = ?
          AND r.computed_at = ?
        "#,
    );

    if params.make.is_some() {
        sql.push_str(" AND COALESCE(v.make, 'unknown') = ? ");
    }
    if params.model.is_some() {
        sql.push_str(" AND COALESCE(v.model, 'unknown') = ? ");
    }
    if params.trim.is_some() {
        sql.push_str(" AND COALESCE(v.trim, 'unknown') = ? ");
    }
    if params.powertrain_class.is_some() {
        sql.push_str(" AND COALESCE(v.powertrain_class, 'unknown') = ? ");
    }

    sql.push_str(" ORDER BY r.rank_position ASC LIMIT ? OFFSET ? ");

    let mut query = sqlx::query(&sql)
        .bind(&params.ranking_type)
        .bind(&timeframe)
        .bind(&temperature_bin)
        .bind(&computed_at);

    if let Some(make) = &params.make {
        query = query.bind(make);
    }
    if let Some(model) = &params.model {
        query = query.bind(model);
    }
    if let Some(trim) = &params.trim {
        query = query.bind(trim);
    }
    if let Some(powertrain_class) = &params.powertrain_class {
        query = query.bind(powertrain_class);
    }

    query = query.bind(limit).bind(offset);

    let rows = query
        .fetch_all(&state.pool)
        .await
        .context("failed to fetch rankings")?;

    let mut ranking_rows = Vec::new();
    let mut cohort = RankingCohort {
        cohort_key: "unknown".to_string(),
        cohort_size: 0,
        sample_gate_passed: false,
    };

    for row in rows {
        let vehicle_uid_str: String = row
            .try_get("vehicle_uid")
            .context("failed to parse ranking vehicle_uid")?;
        let vehicle_uid =
            Uuid::parse_str(&vehicle_uid_str).context("invalid UUID stored in ranking row")?;

        let kpi_rows = sqlx::query(
            r#"
            SELECT kpi_key, kpi_value
            FROM vehicle_kpi_snapshot ks
            WHERE vehicle_uid = ?
              AND ranking_type = ?
              AND timeframe = ?
              AND temperature_bin = ?
              AND computed_at = (
                  SELECT MAX(ks2.computed_at)
                  FROM vehicle_kpi_snapshot ks2
                  WHERE ks2.vehicle_uid = ks.vehicle_uid
                    AND ks2.ranking_type = ks.ranking_type
                    AND ks2.timeframe = ks.timeframe
                    AND ks2.temperature_bin = ks.temperature_bin
                    AND ks2.kpi_key = ks.kpi_key
              )
            "#,
        )
        .bind(&vehicle_uid_str)
        .bind(&params.ranking_type)
        .bind(&timeframe)
        .bind(&temperature_bin)
        .fetch_all(&state.pool)
        .await
        .context("failed to fetch KPI details for ranking row")?;

        let mut kpis = BTreeMap::new();
        for kpi_row in kpi_rows {
            let key: String = kpi_row
                .try_get("kpi_key")
                .context("failed to parse kpi_key in ranking detail")?;
            let value: f64 = kpi_row
                .try_get("kpi_value")
                .context("failed to parse kpi_value in ranking detail")?;
            kpis.insert(key, value);
        }

        cohort = RankingCohort {
            cohort_key: row
                .try_get("cohort_key")
                .context("failed to parse cohort_key")?,
            cohort_size: row
                .try_get("cohort_size")
                .context("failed to parse cohort_size")?,
            sample_gate_passed: row
                .try_get::<i64, _>("sample_gate_passed")
                .context("failed to parse sample_gate_passed")?
                == 1,
        };

        ranking_rows.push(RankingRow {
            rank: row
                .try_get("rank_position")
                .context("failed to parse rank_position")?,
            vehicle_uid,
            score: row.try_get("score").context("failed to parse score")?,
            confidence_level: row
                .try_get("confidence_level")
                .context("failed to parse confidence_level")?,
            kpis,
        });
    }

    let has_more = ranking_rows.len() as i64 >= limit;

    let mut filters = BTreeMap::new();
    filters.insert(
        "powertrain_class".to_string(),
        Some(params.powertrain_class.unwrap_or_else(|| "bev".to_string())),
    );
    filters.insert("make".to_string(), params.make);
    filters.insert("model".to_string(), params.model);
    filters.insert("trim".to_string(), params.trim);
    filters.insert("year_band".to_string(), params.year_band);
    filters.insert("region".to_string(), params.region);

    Ok(Json(RankingsResponse {
        generated_at: now_str(),
        ranking_type: params.ranking_type,
        timeframe,
        temperature_bin,
        filters,
        cohort,
        rows: ranking_rows,
        page: RankingPage {
            limit,
            offset,
            has_more,
        },
    }))
}

async fn run_kpi_job(pool: &SqlitePool) -> Result<JobResponse, ApiError> {
    let job_id = Uuid::new_v4().to_string();

    let charging_sessions_upserted = build_charging_sessions(pool)
        .await
        .context("failed to build charging sessions")?;

    let (kpi_rows_upserted, recomputed_vehicles) = recompute_all_kpis(pool)
        .await
        .context("failed to recompute KPIs")?;

    let ranking_rows_upserted = rebuild_all_rankings(pool)
        .await
        .context("failed to rebuild ranking snapshots for all ranking types")?;

    Ok(JobResponse {
        ok: true,
        job_id,
        charging_sessions_upserted,
        kpi_rows_upserted,
        ranking_rows_upserted,
        recomputed_vehicles,
    })
}

async fn recompute_all_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let (temp_rows, temp_vehicles) = recompute_temperature_kpis(pool).await?;
    let (other_rows, other_vehicles) = recompute_non_temperature_kpis(pool).await?;
    Ok((temp_rows + other_rows, temp_vehicles.max(other_vehicles)))
}

async fn build_charging_sessions(pool: &SqlitePool) -> Result<usize> {
    let session_rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          session_id,
          MIN(CASE WHEN event_type = 'start' THEN observed_at END) AS started_at,
          MAX(CASE WHEN event_type = 'stop' THEN observed_at END) AS ended_at
        FROM vehicle_session_event
        WHERE session_type = 'charging'
        GROUP BY vehicle_uid, session_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read charging session events")?;

    let mut upserted = 0usize;

    for row in session_rows {
        let vehicle_uid: String = row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in session row")?;
        let session_id: String = row
            .try_get("session_id")
            .context("invalid session_id in session row")?;
        let started_at_opt: Option<String> = row
            .try_get("started_at")
            .context("invalid started_at in session row")?;
        let ended_at_opt: Option<String> = row
            .try_get("ended_at")
            .context("invalid ended_at in session row")?;

        let Some(started_at) = started_at_opt else {
            continue;
        };

        let ended_at = ended_at_opt.clone().unwrap_or_else(now_str);

        let obs_rows = sqlx::query(
            r#"
            SELECT signal_key, value_number, value_string, observed_at
            FROM vehicle_signal_observation
            WHERE vehicle_uid = ?
              AND observed_at >= ?
              AND observed_at <= ?
            ORDER BY observed_at ASC
            "#,
        )
        .bind(&vehicle_uid)
        .bind(&started_at)
        .bind(&ended_at)
        .fetch_all(pool)
        .await
        .context("failed to fetch observations for charging session")?;

        let mut soc_series: Vec<(String, f64)> = Vec::new();
        let mut power_series: Vec<f64> = Vec::new();
        let mut ambient_temps = Vec::new();
        let mut battery_temps = Vec::new();
        let mut charger_type = "unknown".to_string();

        for obs in obs_rows {
            let signal_key: String = obs.try_get("signal_key")?;
            let observed_at: String = obs.try_get("observed_at")?;
            let value_number: Option<f64> = obs.try_get("value_number")?;
            let value_string: Option<String> = obs.try_get("value_string")?;

            match signal_key.as_str() {
                "ev.soc_pct" => {
                    if let Some(v) = value_number {
                        soc_series.push((observed_at, v));
                    }
                }
                "ev.charge_power_kw" | "power.battery_power_kw" => {
                    if let Some(v) = value_number {
                        if v.is_finite() {
                            power_series.push(v.abs());
                        }
                    }
                }
                "environment.ambient_temp_c" => {
                    if let Some(v) = value_number {
                        ambient_temps.push(v);
                    }
                }
                "ev.battery_temp_c" => {
                    if let Some(v) = value_number {
                        battery_temps.push(v);
                    }
                }
                "ev.charger_type" => {
                    if let Some(v) = value_string {
                        charger_type = normalize_charger_type(&v).to_string();
                    }
                }
                _ => {}
            }
        }

        soc_series.sort_by(|a, b| a.0.cmp(&b.0));
        let soc_start = soc_series.first().map(|(_, v)| *v);
        let soc_end = soc_series.last().map(|(_, v)| *v);
        let soc_delta = match (soc_start, soc_end) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        };

        let avg_power = mean(&power_series);
        let peak_power = max_value(&power_series);
        let ambient_avg = mean(&ambient_temps);
        let battery_avg = mean(&battery_temps);

        let temperature_source = ambient_avg.or(battery_avg);
        let temperature_bin = temperature_source
            .map(derive_temperature_bin)
            .map(str::to_string);

        let duration_hours = match (
            parse_ts(&started_at),
            ended_at_opt.as_deref().and_then(parse_ts),
        ) {
            (Some(start), Some(end)) if end > start => (end - start).num_seconds() as f64 / 3600.0,
            _ => 0.0,
        };

        let energy_added_kwh = avg_power.map(|p| p * duration_hours.max(0.0));
        let status = if ended_at_opt.is_some() {
            "complete"
        } else {
            "partial"
        };

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
                soc_start_pct,
                soc_end_pct,
                soc_delta_pct,
                energy_added_kwh,
                avg_charge_power_kw,
                peak_charge_power_kw,
                ambient_temp_avg_c,
                battery_temp_avg_c,
                temperature_bin,
                temperature_is_estimated,
                sample_count,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(session_id) DO UPDATE SET
                started_at = excluded.started_at,
                ended_at = excluded.ended_at,
                status = excluded.status,
                charger_type = excluded.charger_type,
                soc_start_pct = excluded.soc_start_pct,
                soc_end_pct = excluded.soc_end_pct,
                soc_delta_pct = excluded.soc_delta_pct,
                energy_added_kwh = excluded.energy_added_kwh,
                avg_charge_power_kw = excluded.avg_charge_power_kw,
                peak_charge_power_kw = excluded.peak_charge_power_kw,
                ambient_temp_avg_c = excluded.ambient_temp_avg_c,
                battery_temp_avg_c = excluded.battery_temp_avg_c,
                temperature_bin = excluded.temperature_bin,
                temperature_is_estimated = excluded.temperature_is_estimated,
                sample_count = excluded.sample_count,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid)
        .bind(&session_id)
        .bind(&started_at)
        .bind(ended_at_opt)
        .bind(status)
        .bind(charger_type)
        .bind(soc_start)
        .bind(soc_end)
        .bind(soc_delta)
        .bind(energy_added_kwh)
        .bind(avg_power)
        .bind(peak_power)
        .bind(ambient_avg)
        .bind(battery_avg)
        .bind(temperature_bin)
        .bind(0_i64)
        .bind(power_series.len() as i64)
        .bind(now_str())
        .bind(now_str())
        .execute(pool)
        .await
        .context("failed to upsert charging session")?;

        upserted += 1;
    }

    Ok(upserted)
}

async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles")?;

    let mut rows_inserted = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        sqlx::query(
            r#"
            DELETE FROM vehicle_kpi_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear KPI snapshots for timeframe {}", timeframe))?;
    }

    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in vehicles list")?;

        for timeframe in ["30d", "90d", "180d"] {
            let cutoff = timeframe_cutoff(timeframe)?;
            let metrics = compute_vehicle_metrics(pool, &vehicle_uid, cutoff).await?;
            let snapshot_ts = now_str();

            for metric in metrics {
                for temp_bin in ["all", "cold"] {
                    insert_kpi_snapshot(
                        pool,
                        &vehicle_uid,
                        "ev_temperature_impact",
                        timeframe,
                        &metric,
                        temp_bin,
                        Some("mild"),
                        Some("cold"),
                        &snapshot_ts,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "failed to insert temperature KPI {} for vehicle {} timeframe {}",
                            metric.key, vehicle_uid, timeframe
                        )
                    })?;

                    rows_inserted += 1;
                }
            }
        }
    }

    Ok((rows_inserted, vehicles.len()))
}

async fn recompute_non_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for non-temperature KPIs")?;

    let mut rows_inserted = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
        ] {
            sqlx::query(
                r#"
                DELETE FROM vehicle_kpi_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear KPI snapshots for ranking_type {} timeframe {}",
                    ranking_type, timeframe
                )
            })?;
        }
    }

    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in non-temperature KPI pass")?;

        for timeframe in ["30d", "90d", "180d"] {
            let cutoff = timeframe_cutoff(timeframe)?;
            let range_metrics =
                compute_range_efficiency_metrics(pool, &vehicle_uid, cutoff).await?;
            let charging_metrics =
                compute_charging_performance_metrics(pool, &vehicle_uid, cutoff).await?;
            let composite_metrics = compute_composite_metrics(&range_metrics, &charging_metrics);

            let snapshot_ts = now_str();
            for metric in &range_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_range_efficiency",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }

            for metric in &charging_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_charging_performance",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }

            for metric in &composite_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_composite",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }
        }
    }

    Ok((rows_inserted, vehicles.len()))
}

async fn insert_kpi_snapshot(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    metric: &MetricCalc,
    temperature_bin: &str,
    baseline_temperature_bin: Option<&str>,
    compare_temperature_bin: Option<&str>,
    snapshot_ts: &str,
) -> Result<()> {
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
            baseline_temperature_bin,
            compare_temperature_bin,
            computed_at,
            source_job_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(metric.key)
    .bind(metric.value)
    .bind(metric.unit)
    .bind(metric.direction)
    .bind(metric.confidence_level)
    .bind(metric.sample_count)
    .bind(temperature_bin)
    .bind(baseline_temperature_bin)
    .bind(compare_temperature_bin)
    .bind(snapshot_ts)
    .bind("internal_recompute")
    .execute(pool)
    .await
    .context("failed to insert KPI snapshot row")?;
    Ok(())
}

async fn rebuild_all_rankings(pool: &SqlitePool) -> Result<usize> {
    let temp_rows = rebuild_temperature_rankings(pool).await?;
    let non_temp_rows = rebuild_non_temperature_rankings(pool).await?;
    Ok(temp_rows + non_temp_rows)
}

async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        let ranking_snapshot_ts = now_str();
        sqlx::query(
            r#"
            DELETE FROM cohort_ranking_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear rankings for timeframe {}", timeframe))?;

        let rows = sqlx::query(
            r#"
            SELECT
              v.vehicle_uid,
              COALESCE(v.make, 'unknown') AS make,
              COALESCE(v.model, 'unknown') AS model,
              COALESCE(v.trim, 'unknown') AS trim,
              v.model_year,
              MAX(CASE WHEN k.kpi_key = 'cold_weather_range_retention' THEN k.kpi_value END) AS range_retention,
              MAX(CASE WHEN k.kpi_key = 'range_temperature_sensitivity_index' THEN k.kpi_value END) AS sensitivity,
              MAX(CASE WHEN k.kpi_key = 'cold_weather_charge_speed_retention' THEN k.kpi_value END) AS charge_retention
            FROM vehicle v
            LEFT JOIN vehicle_kpi_snapshot k
              ON k.vehicle_uid = v.vehicle_uid
             AND k.ranking_type = 'ev_temperature_impact'
             AND k.timeframe = ?
             AND k.temperature_bin = 'cold'
            GROUP BY v.vehicle_uid, v.make, v.model, v.trim, v.model_year
            "#,
        )
        .bind(timeframe)
        .fetch_all(pool)
        .await
        .with_context(|| format!("failed to fetch KPI seeds for timeframe {}", timeframe))?;

        let mut seeds = Vec::new();
        for row in rows {
            let vehicle_uid: String = row.try_get("vehicle_uid")?;
            let make: String = row.try_get("make")?;
            let model: String = row.try_get("model")?;
            let trim: String = row.try_get("trim")?;
            let model_year: Option<i64> = row.try_get("model_year")?;
            let range_retention: Option<f64> = row.try_get("range_retention")?;
            let sensitivity: Option<f64> = row.try_get("sensitivity")?;
            let charge_retention: Option<f64> = row.try_get("charge_retention")?;

            if range_retention.is_none() && sensitivity.is_none() && charge_retention.is_none() {
                continue;
            }

            let confidence_level = match (
                range_retention.is_some(),
                sensitivity.is_some(),
                charge_retention.is_some(),
            ) {
                (true, true, true) => "stable",
                (true, true, false) | (true, false, true) | (false, true, true) => "medium",
                _ => "preview",
            }
            .to_string();

            seeds.push(VehicleRankingSeed {
                vehicle_uid,
                make,
                model,
                trim,
                model_year,
                range_retention,
                sensitivity,
                charge_retention,
                confidence_level,
            });
        }

        let mut cohorts: HashMap<String, Vec<(VehicleRankingSeed, f64)>> = HashMap::new();

        for seed in seeds {
            let score = score_vehicle(&seed);
            let cohort_key = format!(
                "bev|{}|{}|{}|{}",
                seed.make,
                seed.model,
                seed.trim,
                year_band(seed.model_year)
            );
            cohorts.entry(cohort_key).or_default().push((seed, score));
        }

        for (cohort_key, entries) in cohorts {
            let mut entries = entries;
            entries.sort_by(|a, b| cmp_f64_desc(a.1, b.1));
            let cohort_size = entries.len() as i64;
            let sample_gate_passed = cohort_size >= 10;

            for (index, (seed, score)) in entries.into_iter().enumerate() {
                for bin in ["all", "cold"] {
                    sqlx::query(
                        r#"
                        INSERT INTO cohort_ranking_snapshot (
                            ranking_snapshot_id,
                            ranking_type,
                            timeframe,
                            temperature_bin,
                            cohort_key,
                            cohort_size,
                            sample_gate_passed,
                            vehicle_uid,
                            rank_position,
                            score,
                            confidence_level,
                            computed_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind("ev_temperature_impact")
                    .bind(timeframe)
                    .bind(bin)
                    .bind(&cohort_key)
                    .bind(cohort_size)
                    .bind(i64::from(sample_gate_passed))
                    .bind(&seed.vehicle_uid)
                    .bind((index + 1) as i64)
                    .bind(score)
                    .bind(&seed.confidence_level)
                    .bind(&ranking_snapshot_ts)
                    .execute(pool)
                    .await
                    .context("failed to insert cohort ranking snapshot")?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}

async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    let vehicle_rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          COALESCE(make, 'unknown') AS make,
          COALESCE(model, 'unknown') AS model,
          COALESCE(trim, 'unknown') AS trim,
          model_year
        FROM vehicle
        ORDER BY vehicle_uid
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch vehicles for non-temperature rankings")?;

    for timeframe in ["30d", "90d", "180d"] {
        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
        ] {
            sqlx::query(
                r#"
                DELETE FROM cohort_ranking_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear ranking snapshots for {} {}",
                    ranking_type, timeframe
                )
            })?;

            let ranking_snapshot_ts = now_str();
            let mut cohorts: HashMap<String, Vec<(String, f64, String, BTreeMap<String, f64>)>> =
                HashMap::new();

            for row in &vehicle_rows {
                let vehicle_uid: String = row.try_get("vehicle_uid")?;
                let make: String = row.try_get("make")?;
                let model: String = row.try_get("model")?;
                let trim: String = row.try_get("trim")?;
                let model_year: Option<i64> = row.try_get("model_year")?;

                let kpis =
                    fetch_latest_vehicle_kpis(pool, &vehicle_uid, ranking_type, timeframe, "all")
                        .await?;
                if kpis.is_empty() {
                    continue;
                }

                let kpi_map: BTreeMap<String, f64> =
                    kpis.iter().map(|k| (k.kpi_key.clone(), k.value)).collect();

                let score = score_from_kpi_map(ranking_type, &kpi_map);
                let confidence_level = confidence_from_kpi_metrics(&kpis).to_string();
                let cohort_key =
                    format!("bev|{}|{}|{}|{}", make, model, trim, year_band(model_year));

                cohorts.entry(cohort_key).or_default().push((
                    vehicle_uid,
                    score,
                    confidence_level,
                    kpi_map,
                ));
            }

            for (cohort_key, mut entries) in cohorts {
                entries.sort_by(|a, b| cmp_f64_desc(a.1, b.1));
                let cohort_size = entries.len() as i64;
                let sample_gate_passed = cohort_size >= 10;

                for (index, (vehicle_uid, score, confidence_level, _kpis)) in
                    entries.into_iter().enumerate()
                {
                    sqlx::query(
                        r#"
                        INSERT INTO cohort_ranking_snapshot (
                            ranking_snapshot_id,
                            ranking_type,
                            timeframe,
                            temperature_bin,
                            cohort_key,
                            cohort_size,
                            sample_gate_passed,
                            vehicle_uid,
                            rank_position,
                            score,
                            confidence_level,
                            computed_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(ranking_type)
                    .bind(timeframe)
                    .bind("all")
                    .bind(&cohort_key)
                    .bind(cohort_size)
                    .bind(i64::from(sample_gate_passed))
                    .bind(vehicle_uid)
                    .bind((index + 1) as i64)
                    .bind(score)
                    .bind(confidence_level)
                    .bind(&ranking_snapshot_ts)
                    .execute(pool)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to insert non-temperature ranking row for {} {}",
                            ranking_type, timeframe
                        )
                    })?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}

async fn compute_range_efficiency_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
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
    .context("failed to fetch observation rows for range-efficiency KPIs")?;

    #[derive(Default)]
    struct Snapshot {
        odo: Option<f64>,
        soc: Option<f64>,
        temp: Option<f64>,
    }

    let mut by_ts: BTreeMap<DateTime<Utc>, Snapshot> = BTreeMap::new();
    for row in obs_rows {
        let signal_key: String = row.try_get("signal_key")?;
        let value: Option<f64> = row.try_get("value_number")?;
        let observed_at: String = row.try_get("observed_at")?;
        let Some(ts) = parse_ts(&observed_at) else {
            continue;
        };
        let entry = by_ts.entry(ts).or_default();
        match (signal_key.as_str(), value) {
            ("distance.odometer", Some(v)) => entry.odo = Some(v),
            ("ev.soc_pct", Some(v)) => entry.soc = Some(v),
            ("environment.ambient_temp_c", Some(v)) => entry.temp = Some(v),
            _ => {}
        }
    }

    let mut current_odo: Option<f64> = None;
    let mut current_soc: Option<f64> = None;
    let mut current_temp: Option<f64> = None;
    let mut prev_filled: Option<(f64, f64, f64)> = None;

    let mut km_per_soc_points = Vec::new();
    let mut latest_soc: Option<f64> = None;

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
        latest_soc = current_soc;

        if let (Some(odo), Some(soc), Some(temp)) = (current_odo, current_soc, current_temp) {
            if let Some((prev_odo, prev_soc, _prev_temp)) = prev_filled {
                let delta_km = odo - prev_odo;
                let delta_soc = prev_soc - soc;
                if delta_km > 0.05 && delta_soc > 0.05 && delta_soc < 30.0 {
                    let km_per_soc = delta_km / delta_soc;
                    if km_per_soc.is_finite() && km_per_soc > 0.0 && km_per_soc < 50.0 {
                        km_per_soc_points.push(km_per_soc);
                    }
                }
            }
            prev_filled = Some((odo, soc, temp));
        }
    }

    let Some(median_km_per_soc) = median(km_per_soc_points.clone()) else {
        return Ok(Vec::new());
    };

    let mut metrics = Vec::new();
    let sample_count = km_per_soc_points.len() as i64;
    let soc_depletion_per_100km = if median_km_per_soc > 0.0 {
        100.0 / median_km_per_soc
    } else {
        100.0
    };
    let latest_soc = latest_soc.unwrap_or(50.0).clamp(0.0, 100.0);
    let estimated_range = (latest_soc * median_km_per_soc).max(0.0);
    let range_efficiency_score = (100.0 - (soc_depletion_per_100km * 1.2)).clamp(0.0, 100.0);

    metrics.push(MetricCalc {
        key: "ev_estimated_practical_range",
        value: estimated_range,
        unit: "km",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });
    metrics.push(MetricCalc {
        key: "soc_depletion_rate_per_100km",
        value: soc_depletion_per_100km,
        unit: "%_per_100km",
        direction: "lower_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });
    metrics.push(MetricCalc {
        key: "ev_range_efficiency_score",
        value: range_efficiency_score,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    });

    Ok(metrics)
}

async fn compute_charging_performance_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
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

    if let (Some(cold_median), Some(mild_median)) = (median(cold_power.clone()), median(mild_power))
    {
        if mild_median > 0.0 {
            let retention = (100.0 * cold_median / mild_median).clamp(0.0, 200.0);
            let retention_samples = cold_power.len() as i64;
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

fn compute_composite_metrics(
    range_metrics: &[MetricCalc],
    charging_metrics: &[MetricCalc],
) -> Vec<MetricCalc> {
    let range_score = range_metrics
        .iter()
        .find(|m| m.key == "ev_range_efficiency_score")
        .map(|m| m.value);
    let charging_score = charging_metrics
        .iter()
        .find(|m| m.key == "charging_performance_score")
        .map(|m| m.value);

    let Some(composite_value) = (match (range_score, charging_score) {
        (Some(r), Some(c)) => Some((0.6 * r + 0.4 * c).clamp(0.0, 100.0)),
        (Some(r), None) => Some(r.clamp(0.0, 100.0)),
        (None, Some(c)) => Some(c.clamp(0.0, 100.0)),
        (None, None) => None,
    }) else {
        return Vec::new();
    };

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
    );

    vec![MetricCalc {
        key: "ev_composite_score",
        value: composite_value,
        unit: "score",
        direction: "higher_is_better",
        sample_count,
        confidence_level: confidence_from_samples(sample_count),
    }]
}

async fn compute_vehicle_metrics(
    pool: &SqlitePool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<MetricCalc>> {
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
                    }
                }
            }
            prev_filled = Some((odo, soc, temp));
        }
    }

    let mut cold_values = Vec::new();
    let mut mild_values = Vec::new();

    for point in &points {
        if point.temperature_c <= 5.0 {
            cold_values.push(point.km_per_soc);
        }
        if point.temperature_c > 15.0 && point.temperature_c <= 25.0 {
            mild_values.push(point.km_per_soc);
        }
    }

    let cold_median = median(cold_values.clone());
    let mild_median = median(mild_values.clone());

    let mut metrics = Vec::new();

    if let (Some(cold), Some(mild)) = (cold_median, mild_median) {
        if mild > 0.0 {
            let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
            let sample_count = (cold_values.len().min(mild_values.len())) as i64;
            metrics.push(MetricCalc {
                key: "cold_weather_range_retention",
                value: retention,
                unit: "%",
                direction: "higher_is_better",
                sample_count,
                confidence_level: confidence_from_samples(sample_count),
            });

            if points.len() >= 6 {
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

    if let (Some(cold), Some(mild)) = (median(cold_charge.clone()), median(mild_charge.clone())) {
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

    Ok(metrics)
}

fn score_vehicle(seed: &VehicleRankingSeed) -> f64 {
    let range = seed.range_retention.unwrap_or(0.0).clamp(0.0, 200.0);
    let charge = seed.charge_retention.unwrap_or(0.0).clamp(0.0, 200.0);
    let sensitivity = seed.sensitivity.unwrap_or(50.0).clamp(0.0, 100.0);

    let sensitivity_component = (100.0 - (sensitivity * 2.0).clamp(0.0, 100.0)).clamp(0.0, 100.0);

    (0.45 * range + 0.35 * charge + 0.20 * sensitivity_component).clamp(0.0, 100.0)
}

fn score_from_kpi_map(ranking_type: &str, kpis: &BTreeMap<String, f64>) -> f64 {
    match ranking_type {
        "ev_range_efficiency" => kpis
            .get("ev_range_efficiency_score")
            .copied()
            .or_else(|| {
                let est = kpis.get("ev_estimated_practical_range").copied()?;
                let depletion = kpis
                    .get("soc_depletion_rate_per_100km")
                    .copied()
                    .unwrap_or(50.0);
                Some(
                    (0.7 * (est / 5.0).clamp(0.0, 100.0)
                        + 0.3 * (100.0 - depletion).clamp(0.0, 100.0))
                    .clamp(0.0, 100.0),
                )
            })
            .unwrap_or(0.0),
        "ev_charging_performance" => kpis
            .get("charging_performance_score")
            .copied()
            .or_else(|| {
                let acceptance = kpis
                    .get("temp_adjusted_charge_acceptance_score")
                    .copied()
                    .unwrap_or(0.0);
                let retention = kpis
                    .get("cold_weather_charge_speed_retention")
                    .copied()
                    .unwrap_or(acceptance);
                Some((0.6 * acceptance + 0.4 * retention).clamp(0.0, 100.0))
            })
            .unwrap_or(0.0),
        "ev_composite" => kpis
            .get("ev_composite_score")
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 100.0),
        _ => 0.0,
    }
}

fn confidence_from_kpi_metrics(kpis: &[KpiMetric]) -> &'static str {
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
    let schema = include_str!("../schema.sql");

    for statement in schema.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() {
            continue;
        }
        let sql = format!("{};", stmt);
        sqlx::query(&sql)
            .execute(pool)
            .await
            .with_context(|| format!("failed to apply schema statement: {}", stmt))?;
    }

    Ok(())
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

fn derive_temperature_bin(temp_c: f64) -> &'static str {
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

fn normalize_charger_type(value: &str) -> &'static str {
    let lower = value.to_ascii_lowercase();
    if lower.contains("dc") || lower.contains("fast") {
        "dc"
    } else if lower.contains("ac") || lower.contains("level") {
        "ac"
    } else {
        "unknown"
    }
}

fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn read_positive_env(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn max_value(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
}

fn median(mut values: Vec<f64>) -> Option<f64> {
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

fn confidence_from_samples(sample_count: i64) -> &'static str {
    if sample_count >= 60 {
        "stable"
    } else if sample_count >= 20 {
        "medium"
    } else {
        "preview"
    }
}

fn timeframe_cutoff(timeframe: &str) -> Result<DateTime<Utc>> {
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

fn year_band(model_year: Option<i64>) -> String {
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

fn cmp_f64_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
