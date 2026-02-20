use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, IngestResponse, TelemetryBatchRequest};

use super::idempotency_envelope::ExistingBatchEnvelope;
use super::idempotency_response::build_duplicate_ingest_response;

/// Result of idempotency lookup before ingest writes begin.
pub(super) enum IdempotencyOutcome {
    /// No matching batch id exists, so the caller should continue normal writes.
    Fresh { ingest_id: Uuid },
    /// A matching batch id exists with identical envelope, so caller should
    /// short-circuit and return duplicate acknowledgement.
    Duplicate { response: IngestResponse },
}

/// Resolves batch idempotency by validating the envelope fields against any
/// existing `ingest_batch` row with the same `batch_id`.
pub(super) async fn resolve_batch_idempotency(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    source_upper: &str,
) -> Result<IdempotencyOutcome, ApiError> {
    let existing_batch = sqlx::query(
        r#"
        SELECT vehicle_uid, schema_version, source, capture_started_at, capture_ended_at
        FROM ingest_batch
        WHERE batch_id = ?
        LIMIT 1
        "#,
    )
    .bind(payload.batch_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .context("failed to check idempotency")?;

    let ingest_id = Uuid::new_v4();
    let Some(existing_batch_row) = existing_batch else {
        return Ok(IdempotencyOutcome::Fresh { ingest_id });
    };

    let existing_envelope = ExistingBatchEnvelope::from_row(&existing_batch_row)
        .context("failed to parse existing ingest_batch envelope for idempotency check")?;
    let same_envelope = existing_envelope.matches(payload, source_upper);

    if !same_envelope {
        return Err(ApiError::conflict(
            "batch_id already exists with a different payload envelope",
        ));
    }

    Ok(IdempotencyOutcome::Duplicate {
        response: build_duplicate_ingest_response(payload, ingest_id),
    })
}
