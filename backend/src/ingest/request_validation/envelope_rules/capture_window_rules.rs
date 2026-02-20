use crate::{ApiError, TelemetryBatchRequest, read_positive_env};

/// Enforces capture-window ordering and interval bounds from runtime settings.
pub(super) fn validate_capture_window(payload: &TelemetryBatchRequest) -> Result<(), ApiError> {
    if payload.capture_window.ended_at <= payload.capture_window.started_at {
        return Err(ApiError::bad_request(
            "capture_window.ended_at must be after capture_window.started_at",
        ));
    }

    let (min_interval_seconds, max_interval_seconds) = allowed_upload_interval_bounds();

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

    Ok(())
}

fn allowed_upload_interval_bounds() -> (i64, i64) {
    let min_interval_seconds = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MIN_SECONDS", 60);
    let max_interval_candidate = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MAX_SECONDS", 86_400);
    let max_interval_seconds = max_interval_candidate.max(min_interval_seconds);
    (min_interval_seconds, max_interval_seconds)
}
