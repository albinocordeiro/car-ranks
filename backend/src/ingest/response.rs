use uuid::Uuid;

use crate::{IngestRecordError, IngestResponse, TelemetryBatchRequest};

/// Builds the final success payload for a committed telemetry ingest.
pub(super) fn build_ingest_success_response(
    payload: &TelemetryBatchRequest,
    ingest_id: Uuid,
    records_accepted: usize,
    errors: Vec<IngestRecordError>,
) -> IngestResponse {
    IngestResponse {
        accepted: true,
        batch_id: payload.batch_id.clone(),
        ingest_id,
        duplicate: false,
        records_received: payload.records.len(),
        records_accepted,
        records_rejected: payload.records.len().saturating_sub(records_accepted),
        errors,
        next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
    }
}
