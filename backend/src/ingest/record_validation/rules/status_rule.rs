use crate::{IngestRecordError, TelemetryRecord};

use super::ingest_error;

/// Verifies that record status is one of the protocol's allowed enums.
pub(super) fn validate_status(
    record: &TelemetryRecord,
    record_index: usize,
) -> Result<(), IngestRecordError> {
    if !is_supported_status(&record.status) {
        return Err(ingest_error(
            record_index,
            "invalid_status",
            "invalid status enum",
        ));
    }

    Ok(())
}

fn is_supported_status(status: &str) -> bool {
    status == "ok"
        || status == "stale"
        || status == "unavailable"
        || status == "not_supported"
        || status == "permission_denied"
        || status == "error"
}
