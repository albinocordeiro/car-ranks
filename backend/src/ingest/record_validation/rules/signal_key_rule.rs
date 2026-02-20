use std::collections::HashSet;

use crate::{IngestRecordError, TelemetryRecord};

use super::ingest_error;

/// Ensures each record references a known signal from the active registry.
pub(super) fn validate_signal_key(
    record: &TelemetryRecord,
    record_index: usize,
    signal_keys: &HashSet<String>,
) -> Result<(), IngestRecordError> {
    if !signal_keys.contains(&record.signal_key) {
        return Err(ingest_error(
            record_index,
            "unknown_signal_key",
            "signal_key not present in active v0.2 registry",
        ));
    }

    Ok(())
}
