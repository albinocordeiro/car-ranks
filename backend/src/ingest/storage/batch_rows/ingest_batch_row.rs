use anyhow::Context;
use sqlx::{Sqlite, Transaction};

use crate::{ApiError, TelemetryBatchRequest};

/// Persists one ingest-batch envelope row for idempotency and provenance.
pub(super) async fn insert_ingest_batch_row(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    source_upper: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO ingest_batch (
            batch_id,
            vehicle_uid,
            schema_version,
            source,
            capture_started_at,
            capture_ended_at,
            received_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(payload.batch_id.to_string())
    .bind(vehicle_uid)
    .bind(&payload.schema_version)
    .bind(source_upper)
    .bind(payload.capture_window.started_at.to_rfc3339())
    .bind(payload.capture_window.ended_at.to_rfc3339())
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to insert ingest batch")?;

    Ok(())
}
