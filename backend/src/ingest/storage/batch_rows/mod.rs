use sqlx::{Sqlite, Transaction};

use crate::{ApiError, TelemetryBatchRequest};

use self::ingest_batch_row::insert_ingest_batch_row;
use self::vehicle_row::ensure_vehicle_row;

mod ingest_batch_row;
mod vehicle_row;

/// Ensures canonical vehicle/batch rows exist before observation writes.
pub(in crate::ingest) async fn ensure_vehicle_and_batch_rows(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    source_account_id: &str,
    source_upper: &str,
    now: &str,
) -> Result<(), ApiError> {
    ensure_vehicle_row(tx, vehicle_uid, source_account_id, now).await?;
    insert_ingest_batch_row(tx, payload, vehicle_uid, source_upper, now).await?;
    Ok(())
}
