use axum::http::StatusCode;

use crate::{ApiError, TelemetryBatchRequest, read_positive_env};

/// Validates envelope-level ingest rules that do not inspect individual records.
pub(super) fn validate_envelope_basics(
    payload: &TelemetryBatchRequest,
    expected_schema_version: &str,
) -> Result<String, ApiError> {
    let source_upper = payload.source.to_uppercase();
    if source_upper != "OBD" {
        return Err(ApiError::bad_request("source must be OBD for MVP"));
    }

    if payload.schema_version != expected_schema_version {
        return Err(ApiError::bad_request(format!(
            "schema_version must be {} for MVP",
            expected_schema_version
        )));
    }

    if payload.capture_window.ended_at <= payload.capture_window.started_at {
        return Err(ApiError::bad_request(
            "capture_window.ended_at must be after capture_window.started_at",
        ));
    }

    let min_interval_seconds = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MIN_SECONDS", 60);
    let max_interval_candidate = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MAX_SECONDS", 86_400);
    let max_interval_seconds = max_interval_candidate.max(min_interval_seconds);

    if let Some(sample_interval_seconds) = payload.capture_window.sample_interval_seconds {
        if sample_interval_seconds < min_interval_seconds
            || sample_interval_seconds > max_interval_seconds
        {
            return Err(ApiError::bad_request(format!(
                "capture_window.sample_interval_seconds must be between {} and {}",
                min_interval_seconds, max_interval_seconds
            )));
        }
    }

    let capture_window_seconds =
        (payload.capture_window.ended_at - payload.capture_window.started_at).num_seconds();
    if capture_window_seconds > max_interval_seconds {
        return Err(ApiError::bad_request(format!(
            "capture_window duration exceeds maximum allowed {} seconds",
            max_interval_seconds
        )));
    }

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

    if let Some(client) = &payload.client {
        if let Some(platform) = &client.platform {
            if platform.to_lowercase() != "ios" {
                return Err(ApiError::bad_request("client.platform must be ios for MVP"));
            }
        }
    }

    Ok(source_upper)
}
