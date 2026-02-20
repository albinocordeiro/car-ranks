use crate::{ApiError, TelemetryBatchRequest, timestamp_in_capture_window};

/// Validates telemetry record timestamps and optional temperature-bin enums.
pub(super) fn validate_record_rules(payload: &TelemetryBatchRequest) -> Result<(), ApiError> {
    for record in &payload.records {
        if !timestamp_in_capture_window(
            &record.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "records.observed_at must be within capture_window",
            ));
        }
        if let Some(temperature_bin) = &record.temperature_bin {
            let valid_bin = matches!(
                temperature_bin.as_str(),
                "very_cold" | "cold" | "cool" | "mild" | "hot"
            );
            if !valid_bin {
                return Err(ApiError::bad_request(
                    "records.temperature_bin must be one of very_cold,cold,cool,mild,hot",
                ));
            }
        }
    }

    Ok(())
}
