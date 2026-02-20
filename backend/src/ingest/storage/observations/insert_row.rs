use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, TelemetryRecord};

use super::super::super::record_validation::PreparedRecordValues;

/// Persists one validated telemetry record as an observation row.
pub(super) async fn insert_observation_row(
    tx: &mut Transaction<'_, Sqlite>,
    batch_id: &str,
    vehicle_uid: &str,
    record: &TelemetryRecord,
    prepared_record: &PreparedRecordValues,
    now: &str,
    record_index: usize,
) -> Result<(), ApiError> {
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
    .bind(batch_id)
    .bind(&prepared_record.session_id)
    .bind(&record.signal_key)
    .bind(record.value_number)
    .bind(&record.value_string)
    .bind(record.value_bool.map(i64::from))
    .bind(&prepared_record.value_json_text)
    .bind(&record.unit)
    .bind(record.observed_at.to_rfc3339())
    .bind(now)
    .bind("OBD")
    .bind(&record.source_signal)
    .bind(&record.status)
    .bind(record.confidence)
    .bind(record.freshness_ttl_seconds)
    .bind(&prepared_record.derived_temperature_bin)
    .bind(record.is_temperature_estimated.unwrap_or(false) as i64)
    .bind(&record.raw_payload_ref)
    .execute(&mut **tx)
    .await
    .with_context(|| format!("failed to insert observation at index {}", record_index))?;

    Ok(())
}
