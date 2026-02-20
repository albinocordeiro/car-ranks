use anyhow::Context;
use sqlx::{Sqlite, Transaction};

use crate::ApiError;

/// Ensures one canonical vehicle row exists for the payload vehicle UID.
pub(super) async fn ensure_vehicle_row(
    tx: &mut Transaction<'_, Sqlite>,
    vehicle_uid: &str,
    source_account_id: &str,
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

    Ok(())
}
