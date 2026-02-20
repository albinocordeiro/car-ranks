use std::collections::HashSet;

use sqlx::{Sqlite, Transaction};

use crate::{ApiError, IngestRecordError, TelemetryBatchRequest};

use self::insert_row::insert_observation_row;
use super::super::record_validation::validate_and_prepare_record;

mod insert_row;

/// Inserts accepted signal observations and accumulates per-record validation errors.
pub(in crate::ingest) async fn insert_signal_observations(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
    signal_keys: &HashSet<String>,
) -> Result<(usize, Vec<IngestRecordError>), ApiError> {
    let mut errors = Vec::new();
    let mut accepted = 0usize;
    let batch_id = payload.batch_id.to_string();

    for (index, record) in payload.records.iter().enumerate() {
        let prepared_record = match validate_and_prepare_record(record, index, signal_keys) {
            Ok(prepared) => prepared,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };

        insert_observation_row(
            tx,
            &batch_id,
            vehicle_uid,
            record,
            &prepared_record,
            now,
            index,
        )
        .await?;

        accepted += 1;
    }

    Ok((accepted, errors))
}
