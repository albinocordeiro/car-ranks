use axum::http::StatusCode;

use crate::{
    ApiError, TelemetryBatchRequest, map_session_event, read_positive_env,
    timestamp_in_capture_window,
};

/// Sanitized envelope fields that downstream ingest stages depend on.
pub(super) struct ValidatedEnvelope {
    pub(super) source_upper: String,
}

/// Validates request-level ingest rules before opening a DB transaction.
pub(super) fn validate_batch_payload(
    payload: &TelemetryBatchRequest,
) -> Result<ValidatedEnvelope, ApiError> {
    let source_upper = payload.source.to_uppercase();
    if source_upper != "OBD" {
        return Err(ApiError::bad_request("source must be OBD for MVP"));
    }

    if payload.schema_version != super::INGEST_SCHEMA_VERSION {
        return Err(ApiError::bad_request(format!(
            "schema_version must be {} for MVP",
            super::INGEST_SCHEMA_VERSION
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

    Ok(ValidatedEnvelope { source_upper })
}
