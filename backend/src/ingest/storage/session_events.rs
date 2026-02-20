use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, TelemetryBatchRequest, map_session_event};

/// Persists normalized session lifecycle events.
pub(in crate::ingest) async fn insert_session_events(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    for event in &payload.session_events {
        let (session_type, event_type) = map_session_event(&event.event_type)
            .ok_or_else(|| anyhow::anyhow!("unsupported session event type {}", event.event_type))
            .context("failed to map session event")?;

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
    }

    Ok(())
}
