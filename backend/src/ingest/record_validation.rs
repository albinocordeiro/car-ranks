use std::collections::HashSet;

use crate::{IngestRecordError, TelemetryRecord, derive_temperature_bin};

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
    if !signal_keys.contains(&record.signal_key) {
        return Err(IngestRecordError {
            record_index,
            code: "unknown_signal_key".to_string(),
            message: "signal_key not present in active v0.2 registry".to_string(),
        });
    }

    if !(record.status == "ok"
        || record.status == "stale"
        || record.status == "unavailable"
        || record.status == "not_supported"
        || record.status == "permission_denied"
        || record.status == "error")
    {
        return Err(IngestRecordError {
            record_index,
            code: "invalid_status".to_string(),
            message: "invalid status enum".to_string(),
        });
    }

    if let Some(confidence) = record.confidence {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(IngestRecordError {
                record_index,
                code: "invalid_confidence".to_string(),
                message: "confidence must be between 0 and 1".to_string(),
            });
        }
    }

    let value_fields_set = i64::from(record.value_number.is_some())
        + i64::from(record.value_string.is_some())
        + i64::from(record.value_bool.is_some())
        + i64::from(record.value_json.is_some());
    if value_fields_set > 1 {
        return Err(IngestRecordError {
            record_index,
            code: "invalid_value_fields".to_string(),
            message: "only one of value_number/value_string/value_bool/value_json is allowed"
                .to_string(),
        });
    }
    if (record.status == "ok" || record.status == "stale") && value_fields_set == 0 {
        return Err(IngestRecordError {
            record_index,
            code: "missing_value".to_string(),
            message: "status ok/stale requires one value field".to_string(),
        });
    }

    let derived_temperature_bin = record.temperature_bin.clone().or_else(|| {
        match (record.signal_key.as_str(), record.value_number) {
            ("environment.ambient_temp_c", Some(temperature)) => {
                Some(derive_temperature_bin(temperature).to_string())
            }
            _ => None,
        }
    });

    Ok(PreparedRecordValues {
        session_id: record.session_id.map(|id| id.to_string()),
        value_json_text: record.value_json.as_ref().map(|value| value.to_string()),
        derived_temperature_bin,
    })
}
