use uuid::Uuid;

use crate::{IngestResponse, TelemetryBatchRequest};

/// Builds duplicate acknowledgement payload for idempotent replay batches.
pub(super) fn build_duplicate_ingest_response(
    payload: &TelemetryBatchRequest,
    ingest_id: Uuid,
) -> IngestResponse {
    IngestResponse {
        accepted: true,
        batch_id: payload.batch_id,
        ingest_id,
        duplicate: true,
        records_received: payload.records.len(),
        records_accepted: 0,
        records_rejected: 0,
        errors: Vec::new(),
        next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
    }
}
