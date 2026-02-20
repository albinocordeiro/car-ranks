use anyhow::Context;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{ApiError, IngestResponse, TelemetryBatchRequest};

use super::super::idempotency_envelope::ExistingBatchEnvelope;
use super::super::idempotency_response::build_duplicate_ingest_response;

/// Result of PostgreSQL idempotency lookup before ingest writes begin.
pub(super) enum PostgresIdempotencyOutcome {
    Fresh { ingest_id: Uuid },
    Duplicate { response: IngestResponse },
}

/// Resolves `batch_id` idempotency for PostgreSQL ingest.
pub(super) async fn resolve_batch_idempotency_postgres(
    tx: &mut Transaction<'_, Postgres>,
    payload: &TelemetryBatchRequest,
    source_upper: &str,
) -> Result<PostgresIdempotencyOutcome, ApiError> {
    let existing_batch = sqlx::query(
        r#"
        SELECT vehicle_uid, schema_version, source, capture_started_at, capture_ended_at
        FROM ingest_batch
        WHERE batch_id = $1
        LIMIT 1
        "#,
    )
    .bind(payload.batch_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .context("failed to check postgres ingest idempotency")?;

    let ingest_id = Uuid::new_v4();
    let Some(existing_batch_row) = existing_batch else {
        return Ok(PostgresIdempotencyOutcome::Fresh { ingest_id });
    };

    let existing_envelope = ExistingBatchEnvelope::from_pg_row(&existing_batch_row)
        .context("failed to parse existing postgres ingest envelope for idempotency")?;
    let same_envelope = existing_envelope.matches(payload, source_upper);

    if !same_envelope {
        return Err(ApiError::conflict(
            "batch_id already exists with a different payload envelope",
        ));
    }

    Ok(PostgresIdempotencyOutcome::Duplicate {
        response: build_duplicate_ingest_response(payload, ingest_id),
    })
}
