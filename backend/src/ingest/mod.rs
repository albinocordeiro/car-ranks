use anyhow::Context;
use axum::Json;
use axum::extract::State;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    ApiError, AppState, DatabaseBackend, IngestResponse, TelemetryBatchRequest, map_session_event,
    now_str,
};

use self::record_validation::validate_and_prepare_record;
use self::request_validation::validate_batch_payload;

mod record_validation;
mod request_validation;

pub(crate) const INGEST_SCHEMA_VERSION: &str = "0.2";

pub(crate) async fn post_telemetry_batches(
    State(state): State<AppState>,
    Json(payload): Json<TelemetryBatchRequest>,
) -> Result<Json<IngestResponse>, ApiError> {
    if state.backend != DatabaseBackend::Sqlite {
        return Err(crate::errors::postgres_rollout_not_enabled(
            "/v1/telemetry/batches",
        ));
    }

    let validated_envelope = validate_batch_payload(&payload)?;
    let source_upper = validated_envelope.source_upper;

    let source_account_id = payload
        .client
        .as_ref()
        .and_then(|c| c.adapter_fingerprint.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let _client_app_version = payload.client.as_ref().and_then(|c| c.app_version.clone());

    let mut tx = state
        .sqlite_pool
        .begin()
        .await
        .context("failed to open transaction")?;

    // Resolve idempotency before writes so replayed batches can short-circuit safely.
    let existing_batch = sqlx::query(
        r#"
        SELECT vehicle_uid, schema_version, source, capture_started_at, capture_ended_at
        FROM ingest_batch
        WHERE batch_id = ?
        LIMIT 1
        "#,
    )
    .bind(payload.batch_id.to_string())
    .fetch_optional(&mut *tx)
    .await
    .context("failed to check idempotency")?;

    let ingest_id = Uuid::new_v4();

    if let Some(existing_batch) = existing_batch {
        let existing_vehicle_uid: String = existing_batch
            .try_get("vehicle_uid")
            .context("failed to parse existing vehicle_uid for idempotency check")?;
        let existing_schema_version: String = existing_batch
            .try_get("schema_version")
            .context("failed to parse existing schema_version for idempotency check")?;
        let existing_source: String = existing_batch
            .try_get("source")
            .context("failed to parse existing source for idempotency check")?;
        let existing_capture_started_at: String = existing_batch
            .try_get("capture_started_at")
            .context("failed to parse existing capture_started_at for idempotency check")?;
        let existing_capture_ended_at: String = existing_batch
            .try_get("capture_ended_at")
            .context("failed to parse existing capture_ended_at for idempotency check")?;

        let same_envelope = existing_vehicle_uid == payload.vehicle_uid.to_string()
            && existing_schema_version == payload.schema_version
            && existing_source.to_uppercase() == source_upper
            && existing_capture_started_at == payload.capture_window.started_at.to_rfc3339()
            && existing_capture_ended_at == payload.capture_window.ended_at.to_rfc3339();

        if !same_envelope {
            return Err(ApiError::conflict(
                "batch_id already exists with a different payload envelope",
            ));
        }

        tx.commit().await.context("failed to commit duplicate tx")?;
        return Ok(Json(IngestResponse {
            accepted: true,
            batch_id: payload.batch_id,
            ingest_id,
            duplicate: true,
            records_received: payload.records.len(),
            records_accepted: 0,
            records_rejected: 0,
            errors: Vec::new(),
            next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
        }));
    }

    let now = now_str();
    let vehicle_uid_str = payload.vehicle_uid.to_string();

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
    .bind(&vehicle_uid_str)
    .bind(&source_account_id)
    .bind("bev")
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
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
    .bind(&vehicle_uid_str)
    .bind(&payload.schema_version)
    .bind(&source_upper)
    .bind(payload.capture_window.started_at.to_rfc3339())
    .bind(payload.capture_window.ended_at.to_rfc3339())
    .bind(&now)
    .execute(&mut *tx)
    .await
    .context("failed to insert ingest batch")?;

    let mut errors = Vec::new();
    let mut accepted = 0usize;

    for (index, record) in payload.records.iter().enumerate() {
        let prepared_record =
            match validate_and_prepare_record(record, index, state.signal_keys.as_ref()) {
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
        .bind(&vehicle_uid_str)
        .bind(payload.batch_id.to_string())
        .bind(prepared_record.session_id)
        .bind(&record.signal_key)
        .bind(record.value_number)
        .bind(&record.value_string)
        .bind(record.value_bool.map(i64::from))
        .bind(prepared_record.value_json_text)
        .bind(&record.unit)
        .bind(record.observed_at.to_rfc3339())
        .bind(&now)
        .bind("OBD")
        .bind(&record.source_signal)
        .bind(&record.status)
        .bind(record.confidence)
        .bind(record.freshness_ttl_seconds)
        .bind(prepared_record.derived_temperature_bin)
        .bind(record.is_temperature_estimated.unwrap_or(false) as i64)
        .bind(&record.raw_payload_ref)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to insert observation at index {}", index))?;

        accepted += 1;
    }

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
        .bind(&vehicle_uid_str)
        .bind(event.session_id.to_string())
        .bind(session_type)
        .bind(event_type)
        .bind(event.observed_at.to_rfc3339())
        .bind(&now)
        .bind("OBD")
        .bind(None::<String>)
        .execute(&mut *tx)
        .await
        .context("failed to insert session event")?;
    }

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
            .bind(&vehicle_uid_str)
            .bind(payload.batch_id.to_string())
            .bind(if mil_on { "MIL_ON" } else { "MIL_OFF" })
            .bind(diag.observed_at.to_rfc3339())
            .bind(&now)
            .bind("OBD")
            .execute(&mut *tx)
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
                .bind(&vehicle_uid_str)
                .bind(payload.batch_id.to_string())
                .bind("DTC_ACTIVE")
                .bind(code)
                .bind(diag.observed_at.to_rfc3339())
                .bind(&now)
                .bind("OBD")
                .execute(&mut *tx)
                .await
                .context("failed to insert DTC diagnostic event")?;
            }
        }
    }

    tx.commit().await.context("failed to commit ingest tx")?;

    Ok(Json(IngestResponse {
        accepted: true,
        batch_id: payload.batch_id,
        ingest_id,
        duplicate: false,
        records_received: payload.records.len(),
        records_accepted: accepted,
        records_rejected: payload.records.len().saturating_sub(accepted),
        errors,
        next_upload_after_seconds: payload.capture_window.sample_interval_seconds.unwrap_or(60),
    }))
}
