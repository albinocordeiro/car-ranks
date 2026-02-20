use sqlx::{Sqlite, Transaction};

use crate::{ApiError, TelemetryBatchRequest};

use self::insert_row::insert_session_event_row;
use self::normalize::normalize_session_event;

mod insert_row;
mod normalize;

/// Persists normalized session lifecycle events.
pub(in crate::ingest) async fn insert_session_events(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    for event in &payload.session_events {
        let (session_type, event_type) = normalize_session_event(&event.event_type)?;
        insert_session_event_row(tx, vehicle_uid, event, session_type, event_type, now).await?;
    }

    Ok(())
}
