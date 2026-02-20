use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, SessionEventInput};

/// Persists one already-normalized session lifecycle event row.
pub(super) async fn insert_session_event_row(
    tx: &mut Transaction<'_, Sqlite>,
    vehicle_uid: &str,
    event: &SessionEventInput,
    session_type: &str,
    event_type: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO vehicle_session_event (
            session_event_id,
            vehicle_uid,
            session_id,
            session_type,
            event_type,
            observed_at,
            ingested_at,
            source,
            raw_payload_ref
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(event.session_id.to_string())
    .bind(session_type)
    .bind(event_type)
    .bind(event.observed_at.to_rfc3339())
    .bind(now)
    .bind("OBD")
    .bind(None::<String>)
    .execute(&mut **tx)
    .await
    .context("failed to insert session event")?;

    Ok(())
}
