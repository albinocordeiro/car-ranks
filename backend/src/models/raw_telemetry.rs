use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query params for fetching recently ingested raw OBD samples.
#[derive(Debug, Deserialize)]
pub(crate) struct RawTelemetryQuery {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) limit: Option<i64>,
    pub(crate) signal_key: Option<String>,
    pub(crate) include_session_events: Option<bool>,
    pub(crate) batch_id: Option<Uuid>,
    pub(crate) session_id: Option<Uuid>,
    pub(crate) cursor_observed_at: Option<String>,
    pub(crate) cursor_observation_id: Option<String>,
}

/// One raw telemetry row as persisted by ingest.
#[derive(Debug, Serialize)]
pub(crate) struct RawTelemetryRecord {
    pub(crate) observation_id: String,
    pub(crate) batch_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) observed_at: String,
    pub(crate) signal_key: String,
    pub(crate) source_signal: Option<String>,
    pub(crate) status: String,
    pub(crate) value_number: Option<f64>,
    pub(crate) value_string: Option<String>,
    pub(crate) value_bool: Option<bool>,
    pub(crate) value_json: Option<String>,
    pub(crate) raw_payload_ref: Option<String>,
}

/// Response payload for `/v1/telemetry/raw`.
#[derive(Debug, Serialize)]
pub(crate) struct RawTelemetryResponse {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) generated_at: String,
    pub(crate) limit: i64,
    pub(crate) signal_key: Option<String>,
    pub(crate) batch_id: Option<Uuid>,
    pub(crate) session_id: Option<Uuid>,
    pub(crate) include_session_events: bool,
    pub(crate) cursor_observed_at: Option<String>,
    pub(crate) cursor_observation_id: Option<String>,
    pub(crate) next_cursor_observed_at: Option<String>,
    pub(crate) next_cursor_observation_id: Option<String>,
    pub(crate) returned_count: usize,
    pub(crate) rows: Vec<RawTelemetryRecord>,
}
