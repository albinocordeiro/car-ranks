use std::collections::HashSet;

use crate::{IngestRecordError, TelemetryRecord};

use self::derived_fields::derive_temperature_bin_for_record;
use self::rules::validate_record_rules;

mod derived_fields;
mod rules;

/// Insert-ready representation of a telemetry record after validation.
pub(super) struct PreparedRecordValues {
    pub(super) session_id: Option<String>,
    pub(super) value_json_text: Option<String>,
    pub(super) derived_temperature_bin: Option<String>,
}

/// Validates one telemetry record and computes derived fields needed by storage.
pub(super) fn validate_and_prepare_record(
    record: &TelemetryRecord,
    record_index: usize,
    signal_keys: &HashSet<String>,
) -> Result<PreparedRecordValues, IngestRecordError> {
    validate_record_rules(record, record_index, signal_keys)?;

    Ok(PreparedRecordValues {
        session_id: record.session_id.map(|id| id.to_string()),
        value_json_text: record.value_json.as_ref().map(|value| value.to_string()),
        derived_temperature_bin: derive_temperature_bin_for_record(record),
    })
}
