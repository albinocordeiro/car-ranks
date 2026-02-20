use crate::{IngestRecordError, TelemetryRecord};

use super::ingest_error;

/// Checks that optional confidence scores stay within the inclusive [0, 1] range.
pub(super) fn validate_confidence(
    record: &TelemetryRecord,
    record_index: usize,
) -> Result<(), IngestRecordError> {
    if let Some(confidence) = record.confidence {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(ingest_error(
                record_index,
                "invalid_confidence",
                "confidence must be between 0 and 1",
            ));
        }
    }

    Ok(())
}
