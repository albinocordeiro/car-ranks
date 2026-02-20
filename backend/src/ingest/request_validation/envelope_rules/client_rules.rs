use crate::{ApiError, TelemetryBatchRequest};

/// Validates optional client metadata fields gated in the current MVP.
pub(super) fn validate_client_platform(payload: &TelemetryBatchRequest) -> Result<(), ApiError> {
    if let Some(client) = &payload.client {
        if let Some(platform) = &client.platform {
            if !platform.eq_ignore_ascii_case("ios") {
                return Err(ApiError::bad_request("client.platform must be ios for MVP"));
            }
        }
    }

    Ok(())
}
