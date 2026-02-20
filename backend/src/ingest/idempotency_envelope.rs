use anyhow::Result;
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::TelemetryBatchRequest;

/// Persisted ingest-batch envelope fields used for idempotency comparison.
pub(super) struct ExistingBatchEnvelope {
    vehicle_uid: String,
    schema_version: String,
    source: String,
    capture_started_at: String,
    capture_ended_at: String,
}

impl ExistingBatchEnvelope {
    /// Decodes a stored ingest-batch envelope row from PostgreSQL.
    pub(super) fn from_pg_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            vehicle_uid: row.try_get("vehicle_uid")?,
            schema_version: row.try_get("schema_version")?,
            source: row.try_get("source")?,
            capture_started_at: row.try_get("capture_started_at")?,
            capture_ended_at: row.try_get("capture_ended_at")?,
        })
    }

    /// Checks whether a new payload envelope matches the persisted envelope.
    pub(super) fn matches(&self, payload: &TelemetryBatchRequest, source_upper: &str) -> bool {
        self.vehicle_uid == payload.vehicle_uid.to_string()
            && self.schema_version == payload.schema_version
            && self.source.to_uppercase() == source_upper
            && self.capture_started_at == payload.capture_window.started_at.to_rfc3339()
            && self.capture_ended_at == payload.capture_window.ended_at.to_rfc3339()
    }
}
