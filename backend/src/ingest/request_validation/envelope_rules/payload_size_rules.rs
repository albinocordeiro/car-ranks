use axum::http::StatusCode;

use crate::{ApiError, TelemetryBatchRequest};

/// Enforces envelope-level payload size limits and minimum-content constraints.
pub(super) fn validate_batch_size_and_presence(
    payload: &TelemetryBatchRequest,
) -> Result<(), ApiError> {
    if payload.records.len() > 5_000 {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            error: "payload_too_large".to_string(),
            message: "maximum records per batch is 5000".to_string(),
        });
    }

    if payload.records.is_empty()
        && payload.session_events.is_empty()
        && payload.diagnostics.is_empty()
    {
        return Err(ApiError::bad_request(
            "records can only be empty when session_events or diagnostics are present",
        ));
    }

    Ok(())
}
