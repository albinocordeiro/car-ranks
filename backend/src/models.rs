use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct TelemetryBatchRequest {
    pub(crate) batch_id: Uuid,
    pub(crate) schema_version: String,
    pub(crate) vehicle_uid: Uuid,
    pub(crate) source: String,
    pub(crate) client: Option<ClientInfo>,
    pub(crate) capture_window: CaptureWindow,
    #[serde(default)]
    pub(crate) records: Vec<TelemetryRecord>,
    #[serde(default)]
    pub(crate) session_events: Vec<SessionEventInput>,
    #[serde(default)]
    pub(crate) diagnostics: Vec<DiagnosticInput>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientInfo {
    pub(crate) platform: Option<String>,
    pub(crate) app_version: Option<String>,
    pub(crate) adapter_fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CaptureWindow {
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) ended_at: DateTime<Utc>,
    pub(crate) sample_interval_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TelemetryRecord {
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) signal_key: String,
    pub(crate) value_number: Option<f64>,
    pub(crate) value_string: Option<String>,
    pub(crate) value_bool: Option<bool>,
    pub(crate) value_json: Option<Value>,
    pub(crate) unit: Option<String>,
    pub(crate) status: String,
    pub(crate) confidence: Option<f64>,
    pub(crate) source_signal: Option<String>,
    pub(crate) freshness_ttl_seconds: Option<i64>,
    pub(crate) temperature_bin: Option<String>,
    pub(crate) is_temperature_estimated: Option<bool>,
    pub(crate) session_id: Option<Uuid>,
    pub(crate) raw_payload_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SessionEventInput {
    pub(crate) event_type: String,
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) session_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DiagnosticInput {
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) mil_on: Option<bool>,
    pub(crate) dtcs_active: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct IngestRecordError {
    pub(crate) record_index: usize,
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct IngestResponse {
    pub(crate) accepted: bool,
    pub(crate) batch_id: Uuid,
    pub(crate) ingest_id: Uuid,
    pub(crate) duplicate: bool,
    pub(crate) records_received: usize,
    pub(crate) records_accepted: usize,
    pub(crate) records_rejected: usize,
    pub(crate) errors: Vec<IngestRecordError>,
    pub(crate) next_upload_after_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct SamplingConfigResponse {
    pub(crate) generated_at: String,
    pub(crate) platform: String,
    pub(crate) source: String,
    pub(crate) read_only: bool,
    pub(crate) batch_upload: BatchUploadConfig,
    pub(crate) sampling_profiles: Vec<SamplingProfile>,
    pub(crate) kpi_refresh: KpiRefreshConfig,
    pub(crate) feature_flags: FeatureFlags,
}

#[derive(Debug, Serialize)]
pub(crate) struct BatchUploadConfig {
    pub(crate) default_interval_seconds: i64,
    pub(crate) min_interval_seconds: i64,
    pub(crate) max_interval_seconds: i64,
    pub(crate) next_upload_after_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct SamplingProfile {
    pub(crate) mode: String,
    pub(crate) sample_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct KpiRefreshConfig {
    pub(crate) active_vehicle_interval_seconds: i64,
    pub(crate) daily_rebuild_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct FeatureFlags {
    pub(crate) smartcar_enabled: bool,
    pub(crate) remote_commands_enabled: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct JobResponse {
    pub(crate) ok: bool,
    pub(crate) job_id: String,
    pub(crate) charging_sessions_upserted: usize,
    pub(crate) kpi_rows_upserted: usize,
    pub(crate) ranking_rows_upserted: usize,
    pub(crate) recomputed_vehicles: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct KpiTempQuery {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) timeframe: Option<String>,
    pub(crate) baseline_temperature_bin: Option<String>,
    pub(crate) compare_temperature_bin: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct KpiQuery {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) timeframe: Option<String>,
    pub(crate) temperature_bin: Option<String>,
    pub(crate) charger_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct KpiMetric {
    pub(crate) kpi_key: String,
    pub(crate) value: f64,
    pub(crate) unit: String,
    pub(crate) direction: String,
    pub(crate) confidence_level: String,
    pub(crate) sample_count: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct CohortBenchmark {
    pub(crate) cohort_size: usize,
    pub(crate) percentiles: BTreeMap<String, i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemperatureImpactResponse {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) generated_at: String,
    pub(crate) baseline_temperature_bin: String,
    pub(crate) compare_temperature_bin: String,
    pub(crate) metrics: Vec<KpiMetric>,
    pub(crate) cohort_benchmark: CohortBenchmark,
}

#[derive(Debug, Serialize)]
pub(crate) struct GenericKpiResponse {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) generated_at: String,
    pub(crate) timeframe: String,
    pub(crate) temperature_bin: String,
    pub(crate) ranking_type: String,
    pub(crate) kpis: Vec<KpiMetric>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RankingsQuery {
    pub(crate) ranking_type: String,
    pub(crate) timeframe: Option<String>,
    pub(crate) temperature_bin: Option<String>,
    pub(crate) powertrain_class: Option<String>,
    pub(crate) make: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) trim: Option<String>,
    pub(crate) year_band: Option<String>,
    pub(crate) region: Option<String>,
    pub(crate) limit: Option<i64>,
    pub(crate) offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingRow {
    pub(crate) rank: i64,
    pub(crate) vehicle_uid: Uuid,
    pub(crate) score: f64,
    pub(crate) confidence_level: String,
    pub(crate) kpis: BTreeMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingPage {
    pub(crate) limit: i64,
    pub(crate) offset: i64,
    pub(crate) has_more: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingCohort {
    pub(crate) cohort_key: String,
    pub(crate) cohort_size: i64,
    pub(crate) sample_gate_passed: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingsResponse {
    pub(crate) generated_at: String,
    pub(crate) ranking_type: String,
    pub(crate) timeframe: String,
    pub(crate) temperature_bin: String,
    pub(crate) filters: BTreeMap<String, Option<String>>,
    pub(crate) cohort: RankingCohort,
    pub(crate) rows: Vec<RankingRow>,
    pub(crate) page: RankingPage,
}

#[derive(Debug)]
pub(crate) struct MetricCalc {
    pub(crate) key: &'static str,
    pub(crate) value: f64,
    pub(crate) unit: &'static str,
    pub(crate) direction: &'static str,
    pub(crate) sample_count: i64,
    pub(crate) confidence_level: &'static str,
}
