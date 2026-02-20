use sqlx::{Sqlite, Transaction};

use crate::{ApiError, TelemetryBatchRequest};

use self::dtc_rows::insert_active_dtc_events;
use self::mil_rows::insert_mil_event;

mod dtc_rows;
mod mil_rows;

/// Persists diagnostic events (MIL state changes and active DTC snapshots).
pub(in crate::ingest) async fn insert_diagnostic_events(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    let batch_id = payload.batch_id.to_string();

    for diagnostic in &payload.diagnostics {
        let observed_at = diagnostic.observed_at.to_rfc3339();

        if let Some(mil_on) = diagnostic.mil_on {
            insert_mil_event(tx, vehicle_uid, &batch_id, mil_on, &observed_at, now).await?;
        }

        if let Some(dtcs) = &diagnostic.dtcs_active {
            insert_active_dtc_events(tx, vehicle_uid, &batch_id, dtcs, &observed_at, now).await?;
        }
    }

    Ok(())
}
