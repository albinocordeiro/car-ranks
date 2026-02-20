use crate::{ApiError, TelemetryBatchRequest};

use self::envelope_rules::validate_envelope_basics;
use self::payload_rules::validate_payload_records_and_events;

mod envelope_rules;
mod payload_rules;

/// Sanitized envelope fields that downstream ingest stages depend on.
pub(super) struct ValidatedEnvelope {
    pub(super) source_upper: String,
}

/// Validates request-level ingest rules before opening a DB transaction.
pub(super) fn validate_batch_payload(
    payload: &TelemetryBatchRequest,
) -> Result<ValidatedEnvelope, ApiError> {
    let source_upper = validate_envelope_basics(payload, super::INGEST_SCHEMA_VERSION)?;
    validate_payload_records_and_events(payload)?;
    Ok(ValidatedEnvelope { source_upper })
}
