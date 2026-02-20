use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use super::{DiagnosticInput, SessionEventInput};

/// Top-level telemetry upload envelope for `/v1/telemetry/batches`.
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
