use crate::{ApiError, TelemetryBatchRequest, map_session_event, timestamp_in_capture_window};

/// Validates session event type mapping and capture-window timestamp bounds.
pub(super) fn validate_session_event_rules(
    payload: &TelemetryBatchRequest,
) -> Result<(), ApiError> {
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

    Ok(())
}

/// Validates diagnostic event timestamps against the capture window.
pub(super) fn validate_diagnostic_timestamps(
    payload: &TelemetryBatchRequest,
) -> Result<(), ApiError> {
    for diagnostic in &payload.diagnostics {
        if !timestamp_in_capture_window(
            &diagnostic.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "diagnostics.observed_at must be within capture_window",
            ));
        }
    }

    Ok(())
}
