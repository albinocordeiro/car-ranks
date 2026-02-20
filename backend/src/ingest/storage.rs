use std::collections::HashSet;

use anyhow::Context;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{ApiError, IngestRecordError, TelemetryBatchRequest, map_session_event};

use super::record_validation::validate_and_prepare_record;

/// Ensures canonical vehicle/batch rows exist before observation writes.
pub(super) async fn ensure_vehicle_and_batch_rows(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    source_account_id: &str,
    source_upper: &str,
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

    sqlx::query(
        r#"
        INSERT INTO ingest_batch (
            batch_id,
            vehicle_uid,
            schema_version,
            source,
            capture_started_at,
            capture_ended_at,
            received_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(payload.batch_id.to_string())
    .bind(vehicle_uid)
    .bind(&payload.schema_version)
    .bind(source_upper)
    .bind(payload.capture_window.started_at.to_rfc3339())
    .bind(payload.capture_window.ended_at.to_rfc3339())
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to insert ingest batch")?;

    Ok(())
}

/// Inserts accepted signal observations and accumulates per-record validation errors.
pub(super) async fn insert_signal_observations(
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

/// Persists normalized session lifecycle events.
pub(super) async fn insert_session_events(
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

/// Persists diagnostic events (MIL state changes and active DTC snapshots).
pub(super) async fn insert_diagnostic_events(
    tx: &mut Transaction<'_, Sqlite>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    for diag in &payload.diagnostics {
        if let Some(mil_on) = diag.mil_on {
            sqlx::query(
                r#"
                INSERT INTO vehicle_diagnostic_event (
                    event_id,
                    vehicle_uid,
                    batch_id,
                    event_type,
                    observed_at,
                    ingested_at,
                    source
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(vehicle_uid)
            .bind(payload.batch_id.to_string())
            .bind(if mil_on { "MIL_ON" } else { "MIL_OFF" })
            .bind(diag.observed_at.to_rfc3339())
            .bind(now)
            .bind("OBD")
            .execute(&mut **tx)
            .await
            .context("failed to insert MIL diagnostic event")?;
        }

        if let Some(dtcs) = &diag.dtcs_active {
            for code in dtcs {
                sqlx::query(
                    r#"
                    INSERT INTO vehicle_diagnostic_event (
                        event_id,
                        vehicle_uid,
                        batch_id,
                        event_type,
                        code,
                        observed_at,
                        ingested_at,
                        source
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(vehicle_uid)
                .bind(payload.batch_id.to_string())
                .bind("DTC_ACTIVE")
                .bind(code)
                .bind(diag.observed_at.to_rfc3339())
                .bind(now)
                .bind("OBD")
                .execute(&mut **tx)
                .await
                .context("failed to insert DTC diagnostic event")?;
            }
        }
    }

    Ok(())
}
