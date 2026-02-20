use std::collections::HashSet;

use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, IngestRecordError, TelemetryBatchRequest};

use super::super::record_validation::validate_and_prepare_record;

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

    for (index, record) in payload.records.iter().enumerate() {
        let prepared_record = match validate_and_prepare_record(record, index, signal_keys) {
            Ok(prepared) => prepared,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };

        sqlx::query(
            r#"
            INSERT INTO vehicle_signal_observation (
                observation_id,
                vehicle_uid,
                batch_id,
                session_id,
                signal_key,
                value_number,
                value_string,
                value_bool,
                value_json,
                unit,
                observed_at,
                ingested_at,
                source,
                source_signal,
                status,
                confidence,
                freshness_ttl_seconds,
                temperature_bin,
                is_temperature_estimated,
                raw_payload_ref
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(vehicle_uid)
        .bind(payload.batch_id.to_string())
        .bind(prepared_record.session_id)
        .bind(&record.signal_key)
        .bind(record.value_number)
        .bind(&record.value_string)
        .bind(record.value_bool.map(i64::from))
        .bind(prepared_record.value_json_text)
        .bind(&record.unit)
        .bind(record.observed_at.to_rfc3339())
        .bind(now)
        .bind("OBD")
        .bind(&record.source_signal)
        .bind(&record.status)
        .bind(record.confidence)
        .bind(record.freshness_ttl_seconds)
        .bind(prepared_record.derived_temperature_bin)
        .bind(record.is_temperature_estimated.unwrap_or(false) as i64)
        .bind(&record.raw_payload_ref)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("failed to insert observation at index {}", index))?;

        accepted += 1;
    }

    Ok((accepted, errors))
}
