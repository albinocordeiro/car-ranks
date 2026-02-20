use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::ApiError;

/// Persists one MIL state diagnostic event.
pub(super) async fn insert_mil_event(
    tx: &mut Transaction<'_, Sqlite>,
    vehicle_uid: &str,
    batch_id: &str,
    mil_on: bool,
    observed_at: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO vehicle_diagnostic_event (
            event_id,
            vehicle_uid,
            batch_id,
            event_type,
            observed_at,
            ingested_at,
            source
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(batch_id)
    .bind(if mil_on { "MIL_ON" } else { "MIL_OFF" })
    .bind(observed_at)
    .bind(now)
    .bind("OBD")
    .execute(&mut **tx)
    .await
    .context("failed to insert MIL diagnostic event")?;

    Ok(())
}
