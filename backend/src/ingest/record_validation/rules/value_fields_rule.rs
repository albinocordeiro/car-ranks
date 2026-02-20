use crate::{IngestRecordError, TelemetryRecord};

use super::ingest_error;

/// Enforces value cardinality and status-coupled value presence constraints.
pub(super) fn validate_value_fields(
    record: &TelemetryRecord,
    record_index: usize,
) -> Result<(), IngestRecordError> {
    let value_fields_set = i64::from(record.value_number.is_some())
        + i64::from(record.value_string.is_some())
        + i64::from(record.value_bool.is_some())
        + i64::from(record.value_json.is_some());

    if value_fields_set > 1 {
        return Err(ingest_error(
            record_index,
            "invalid_value_fields",
            "only one of value_number/value_string/value_bool/value_json is allowed",
        ));
    }

    if requires_value_payload(record.status.as_str()) && value_fields_set == 0 {
        return Err(ingest_error(
            record_index,
            "missing_value",
            "status ok/stale requires one value field",
        ));
    }

    Ok(())
}

fn requires_value_payload(status: &str) -> bool {
    status == "ok" || status == "stale"
}
