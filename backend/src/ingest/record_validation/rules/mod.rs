use std::collections::HashSet;

use crate::{IngestRecordError, TelemetryRecord};

mod confidence_rule;
mod signal_key_rule;
mod status_rule;
mod value_fields_rule;

use confidence_rule::validate_confidence;
use signal_key_rule::validate_signal_key;
use status_rule::validate_status;
use value_fields_rule::validate_value_fields;

/// Applies all record-level ingest validation rules in deterministic order.
///
/// The ordering intentionally stays stable so clients get predictable first-failure
/// behavior when a record violates multiple constraints at once.
pub(super) fn validate_record_rules(
    record: &TelemetryRecord,
    record_index: usize,
    signal_keys: &HashSet<String>,
) -> Result<(), IngestRecordError> {
    validate_signal_key(record, record_index, signal_keys)?;
    validate_status(record, record_index)?;
    validate_confidence(record, record_index)?;
    validate_value_fields(record, record_index)?;
    Ok(())
}

/// Creates a normalized ingest-record validation error.
pub(super) fn ingest_error(record_index: usize, code: &str, message: &str) -> IngestRecordError {
    IngestRecordError {
        record_index,
        code: code.to_string(),
        message: message.to_string(),
    }
}
