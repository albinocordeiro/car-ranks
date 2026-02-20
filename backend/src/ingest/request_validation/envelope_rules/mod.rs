use crate::{ApiError, TelemetryBatchRequest};

mod capture_window_rules;
mod client_rules;
mod payload_size_rules;
mod source_rules;

use capture_window_rules::validate_capture_window;
use client_rules::validate_client_platform;
use payload_size_rules::validate_batch_size_and_presence;
use source_rules::validate_source_and_schema;

/// Validates envelope-level ingest rules that do not inspect individual records.
pub(super) fn validate_envelope_basics(
    payload: &TelemetryBatchRequest,
    expected_schema_version: &str,
) -> Result<String, ApiError> {
    let source_upper = validate_source_and_schema(payload, expected_schema_version)?;
    validate_capture_window(payload)?;
    validate_batch_size_and_presence(payload)?;
    validate_client_platform(payload)?;
    Ok(source_upper)
}
