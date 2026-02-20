use std::collections::HashSet;

use crate::{IngestRecordError, TelemetryRecord};

/// Applies all record-level ingest validation rules in deterministic order.
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

fn validate_signal_key(
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

fn validate_status(record: &TelemetryRecord, record_index: usize) -> Result<(), IngestRecordError> {
    if !(record.status == "ok"
        || record.status == "stale"
        || record.status == "unavailable"
        || record.status == "not_supported"
        || record.status == "permission_denied"
        || record.status == "error")
    {
        return Err(ingest_error(
            record_index,
            "invalid_status",
            "invalid status enum",
        ));
    }
    Ok(())
}

fn validate_confidence(
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

fn validate_value_fields(
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

    if (record.status == "ok" || record.status == "stale") && value_fields_set == 0 {
        return Err(ingest_error(
            record_index,
            "missing_value",
            "status ok/stale requires one value field",
        ));
    }

    Ok(())
}

fn ingest_error(record_index: usize, code: &str, message: &str) -> IngestRecordError {
    IngestRecordError {
        record_index,
        code: code.to_string(),
        message: message.to_string(),
    }
}
