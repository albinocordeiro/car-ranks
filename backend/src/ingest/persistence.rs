use std::collections::HashSet;

use sqlx::{Sqlite, Transaction};

use crate::{ApiError, IngestRecordError, TelemetryBatchRequest};

use super::source_context::SourceContext;
use super::storage::{
    ensure_vehicle_and_batch_rows, insert_diagnostic_events, insert_session_events,
    insert_signal_observations,
};

/// Persists all validated ingest payload rows inside the active transaction.
pub(super) async fn persist_validated_batch(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    source_context: &SourceContext,
    now: &str,
    signal_keys: &HashSet<String>,
) -> Result<(usize, Vec<IngestRecordError>), ApiError> {
    let vehicle_uid = payload.vehicle_uid.to_string();

    ensure_vehicle_and_batch_rows(
        tx,
        payload,
        &vehicle_uid,
        &source_context.source_account_id,
        &source_context.source_upper,
        now,
    )
    .await?;

    let (accepted, errors) =
        insert_signal_observations(tx, payload, &vehicle_uid, now, signal_keys).await?;
    insert_session_events(tx, payload, &vehicle_uid, now).await?;
    insert_diagnostic_events(tx, payload, &vehicle_uid, now).await?;

    Ok((accepted, errors))
}
