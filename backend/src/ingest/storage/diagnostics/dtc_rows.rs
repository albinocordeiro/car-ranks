use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::ApiError;

/// Persists active DTC diagnostic events for one observation timestamp.
pub(super) async fn insert_active_dtc_events(
    tx: &mut Transaction<'_, Sqlite>,
    vehicle_uid: &str,
    batch_id: &str,
    dtcs: &[String],
    observed_at: &str,
    now: &str,
) -> Result<(), ApiError> {
    for code in dtcs {
        sqlx::query(
            r#"
            INSERT INTO vehicle_diagnostic_event (
                event_id,
                vehicle_uid,
                batch_id,
                event_type,
                code,
                observed_at,
                ingested_at,
                source
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(vehicle_uid)
        .bind(batch_id)
        .bind("DTC_ACTIVE")
        .bind(code)
        .bind(observed_at)
        .bind(now)
        .bind("OBD")
        .execute(&mut **tx)
        .await
        .context("failed to insert DTC diagnostic event")?;
    }

    Ok(())
}
