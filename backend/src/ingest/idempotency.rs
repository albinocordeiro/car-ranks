use anyhow::Context;
use sqlx::Row;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, IngestResponse, TelemetryBatchRequest};

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
    let Some(existing_batch) = existing_batch else {
        return Ok(IdempotencyOutcome::Fresh { ingest_id });
    };

    let existing_vehicle_uid: String = existing_batch
        .try_get("vehicle_uid")
        .context("failed to parse existing vehicle_uid for idempotency check")?;
    let existing_schema_version: String = existing_batch
        .try_get("schema_version")
        .context("failed to parse existing schema_version for idempotency check")?;
    let existing_source: String = existing_batch
        .try_get("source")
        .context("failed to parse existing source for idempotency check")?;
    let existing_capture_started_at: String = existing_batch
        .try_get("capture_started_at")
        .context("failed to parse existing capture_started_at for idempotency check")?;
    let existing_capture_ended_at: String = existing_batch
        .try_get("capture_ended_at")
        .context("failed to parse existing capture_ended_at for idempotency check")?;

    let same_envelope = existing_vehicle_uid == payload.vehicle_uid.to_string()
        && existing_schema_version == payload.schema_version
        && existing_source.to_uppercase() == source_upper
        && existing_capture_started_at == payload.capture_window.started_at.to_rfc3339()
        && existing_capture_ended_at == payload.capture_window.ended_at.to_rfc3339();

    if !same_envelope {
        return Err(ApiError::conflict(
            "batch_id already exists with a different payload envelope",
        ));
    }

    Ok(IdempotencyOutcome::Duplicate {
        response: IngestResponse {
            accepted: true,
            batch_id: payload.batch_id,
            ingest_id,
            duplicate: true,
            records_received: payload.records.len(),
            records_accepted: 0,
            records_rejected: 0,
            errors: Vec::new(),
            next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
        },
    })
}
