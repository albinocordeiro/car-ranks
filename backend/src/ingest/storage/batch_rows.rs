use anyhow::Context;
use sqlx::{Sqlite, Transaction};

use crate::{ApiError, TelemetryBatchRequest};

/// Ensures canonical vehicle/batch rows exist before observation writes.
pub(in crate::ingest) async fn ensure_vehicle_and_batch_rows(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    source_account_id: &str,
    source_upper: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO vehicle (
            vehicle_uid,
            source_account_id,
            powertrain_class,
            created_at,
            updated_at
        ) VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(vehicle_uid)
    .bind(source_account_id)
    .bind("bev")
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to ensure vehicle")?;

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
