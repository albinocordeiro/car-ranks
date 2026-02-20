use crate::{ApiError, TelemetryBatchRequest};

use self::event_rules::{validate_diagnostic_timestamps, validate_session_event_rules};
use self::record_rules::validate_record_rules;

mod event_rules;
mod record_rules;

/// Validates per-item payload rules (timestamps, enum values, and temperature bins).
pub(super) fn validate_payload_records_and_events(
    payload: &TelemetryBatchRequest,
) -> Result<(), ApiError> {
    // Validate envelope timestamps before opening a write transaction.
    validate_session_event_rules(payload)?;
    validate_diagnostic_timestamps(payload)?;
    validate_record_rules(payload)?;
    Ok(())
}
