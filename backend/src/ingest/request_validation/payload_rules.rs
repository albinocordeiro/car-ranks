use crate::{ApiError, TelemetryBatchRequest, map_session_event, timestamp_in_capture_window};

/// Validates per-item payload rules (timestamps, enum values, and temperature bins).
pub(super) fn validate_payload_records_and_events(
    payload: &TelemetryBatchRequest,
) -> Result<(), ApiError> {
    // Validate envelope timestamps before opening a write transaction.
    for event in &payload.session_events {
        if map_session_event(&event.event_type).is_none() {
            return Err(ApiError::bad_request(format!(
                "unsupported session_events.event_type: {}",
                event.event_type
            )));
        }
        if !timestamp_in_capture_window(
            &event.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "session_events.observed_at must be within capture_window",
            ));
        }
    }

    for diag in &payload.diagnostics {
        if !timestamp_in_capture_window(
            &diag.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "diagnostics.observed_at must be within capture_window",
            ));
        }
    }

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
