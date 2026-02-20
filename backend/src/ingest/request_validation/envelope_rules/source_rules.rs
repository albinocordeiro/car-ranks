use crate::{ApiError, TelemetryBatchRequest};

/// Validates ingest source and schema fields used to gate protocol compatibility.
pub(super) fn validate_source_and_schema(
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

    Ok(source_upper)
}
