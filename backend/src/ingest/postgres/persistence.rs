use std::collections::HashSet;

use anyhow::Context;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::bind_vehicle_owner_postgres;
use crate::{
    ApiError, IngestRecordError, SessionEventInput, TelemetryBatchRequest, TelemetryRecord,
    map_session_event,
};

use super::super::record_validation::{PreparedRecordValues, validate_and_prepare_record};
use super::super::source_context::SourceContext;

/// Persists a validated ingest payload into PostgreSQL storage.
pub(super) async fn persist_validated_batch_postgres(
    tx: &mut Transaction<'_, Postgres>,
    payload: &TelemetryBatchRequest,
    source_context: &SourceContext,
    auth_user_id: &str,
    now: &str,
    signal_keys: &HashSet<String>,
) -> Result<(usize, Vec<IngestRecordError>), ApiError> {
    let vehicle_uid = payload.vehicle_uid.to_string();
    ensure_vehicle_and_batch_rows_postgres(
        tx,
        payload,
        auth_user_id,
        &vehicle_uid,
        &source_context.source_account_id,
        &source_context.source_upper,
        now,
    )
    .await?;

    let (accepted, errors) =
        insert_signal_observations_postgres(tx, payload, &vehicle_uid, now, signal_keys).await?;
    insert_session_events_postgres(tx, payload, &vehicle_uid, now).await?;
    insert_diagnostic_events_postgres(tx, payload, &vehicle_uid, now).await?;

    Ok((accepted, errors))
}

/// Ensures canonical vehicle and ingest-batch rows exist in PostgreSQL.
async fn ensure_vehicle_and_batch_rows_postgres(
    tx: &mut Transaction<'_, Postgres>,
    payload: &TelemetryBatchRequest,
    auth_user_id: &str,
    vehicle_uid: &str,
    source_account_id: &str,
    source_upper: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO vehicle (
            vehicle_uid,
            source_account_id,
            powertrain_class,
            created_at,
            updated_at
        ) VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (vehicle_uid) DO NOTHING
        "#,
    )
    .bind(vehicle_uid)
    .bind(source_account_id)
    .bind("bev")
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .context("failed to ensure postgres vehicle row")?;

    bind_vehicle_owner_postgres(tx, auth_user_id, vehicle_uid, now).await?;

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
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
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
    .context("failed to insert postgres ingest batch row")?;

    Ok(())
}

/// Inserts accepted observation rows and accumulates per-record validation errors.
async fn insert_signal_observations_postgres(
    tx: &mut Transaction<'_, Postgres>,
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

        insert_observation_row_postgres(
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

/// Persists one validated observation row into PostgreSQL storage.
async fn insert_observation_row_postgres(
    tx: &mut Transaction<'_, Postgres>,
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
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
        )
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
    .with_context(|| {
        format!(
            "failed to insert postgres observation at index {}",
            record_index
        )
    })?;

    Ok(())
}

/// Persists normalized session lifecycle events into PostgreSQL storage.
async fn insert_session_events_postgres(
    tx: &mut Transaction<'_, Postgres>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    for event in &payload.session_events {
        let (session_type, event_type) = normalize_session_event(event)?;
        insert_session_event_row_postgres(tx, vehicle_uid, event, session_type, event_type, now)
            .await?;
    }

    Ok(())
}

fn normalize_session_event(
    event: &SessionEventInput,
) -> Result<(&'static str, &'static str), ApiError> {
    map_session_event(&event.event_type).ok_or_else(|| {
        ApiError::bad_request(format!(
            "unsupported session event type {}",
            event.event_type
        ))
    })
}

/// Persists one normalized session lifecycle event row.
async fn insert_session_event_row_postgres(
    tx: &mut Transaction<'_, Postgres>,
    vehicle_uid: &str,
    event: &SessionEventInput,
    session_type: &str,
    event_type: &str,
    now: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO vehicle_session_event (
            session_event_id,
            vehicle_uid,
            session_id,
            session_type,
            event_type,
            observed_at,
            ingested_at,
            source,
            raw_payload_ref
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (vehicle_uid, session_id, session_type, event_type, observed_at) DO NOTHING
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
    .bind(&event.raw_payload_ref)
    .execute(&mut **tx)
    .await
    .context("failed to insert postgres session event")?;

    Ok(())
}

/// Persists diagnostic events into PostgreSQL storage.
async fn insert_diagnostic_events_postgres(
    tx: &mut Transaction<'_, Postgres>,
    payload: &TelemetryBatchRequest,
    vehicle_uid: &str,
    now: &str,
) -> Result<(), ApiError> {
    let batch_id = payload.batch_id.to_string();
    for diagnostic in &payload.diagnostics {
        let observed_at = diagnostic.observed_at.to_rfc3339();

        if let Some(mil_on) = diagnostic.mil_on {
            insert_mil_event_postgres(tx, vehicle_uid, &batch_id, mil_on, &observed_at, now)
                .await?;
        }

        if let Some(dtcs) = &diagnostic.dtcs_active {
            insert_active_dtc_events_postgres(tx, vehicle_uid, &batch_id, dtcs, &observed_at, now)
                .await?;
        }
    }

    Ok(())
}

async fn insert_mil_event_postgres(
    tx: &mut Transaction<'_, Postgres>,
    vehicle_uid: &str,
    batch_id: &str,
    mil_on: bool,
    observed_at: &str,
    now: &str,
) -> Result<(), ApiError> {
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
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(batch_id)
    .bind(if mil_on { "MIL_ON" } else { "MIL_OFF" })
    .bind(observed_at)
    .bind(now)
    .bind("OBD")
    .execute(&mut **tx)
    .await
    .context("failed to insert postgres MIL diagnostic event")?;

    Ok(())
}

async fn insert_active_dtc_events_postgres(
    tx: &mut Transaction<'_, Postgres>,
    vehicle_uid: &str,
    batch_id: &str,
    dtcs: &[String],
    observed_at: &str,
    now: &str,
) -> Result<(), ApiError> {
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(vehicle_uid)
        .bind(batch_id)
        .bind("DTC_ACTIVE")
        .bind(code)
        .bind(observed_at)
        .bind(now)
        .bind("OBD")
        .execute(&mut **tx)
        .await
        .context("failed to insert postgres DTC diagnostic event")?;
    }

    Ok(())
}
