use anyhow::Context;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    ApiError, AppState, DatabaseBackend, IngestRecordError, IngestResponse, TelemetryBatchRequest,
    derive_temperature_bin, map_session_event, now_str, postgres_rollout_not_enabled,
    read_positive_env, timestamp_in_capture_window,
};

pub(crate) const INGEST_SCHEMA_VERSION: &str = "0.2";

pub(crate) async fn post_telemetry_batches(
    State(state): State<AppState>,
    Json(payload): Json<TelemetryBatchRequest>,
) -> Result<Json<IngestResponse>, ApiError> {
    if state.backend != DatabaseBackend::Sqlite {
        return Err(postgres_rollout_not_enabled("/v1/telemetry/batches"));
    }

    let source_upper = payload.source.to_uppercase();
    if source_upper != "OBD" {
        return Err(ApiError::bad_request("source must be OBD for MVP"));
    }

    if payload.schema_version != INGEST_SCHEMA_VERSION {
        return Err(ApiError::bad_request(format!(
            "schema_version must be {} for MVP",
            INGEST_SCHEMA_VERSION
        )));
    }

    if payload.capture_window.ended_at <= payload.capture_window.started_at {
        return Err(ApiError::bad_request(
            "capture_window.ended_at must be after capture_window.started_at",
        ));
    }

    let min_interval_seconds = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MIN_SECONDS", 60);
    let max_interval_candidate = read_positive_env("CAR_RANKS_UPLOAD_INTERVAL_MAX_SECONDS", 86_400);
    let max_interval_seconds = max_interval_candidate.max(min_interval_seconds);

    if let Some(sample_interval_seconds) = payload.capture_window.sample_interval_seconds {
        if sample_interval_seconds < min_interval_seconds
            || sample_interval_seconds > max_interval_seconds
        {
            return Err(ApiError::bad_request(format!(
                "capture_window.sample_interval_seconds must be between {} and {}",
                min_interval_seconds, max_interval_seconds
            )));
        }
    }

    let capture_window_seconds =
        (payload.capture_window.ended_at - payload.capture_window.started_at).num_seconds();
    if capture_window_seconds > max_interval_seconds {
        return Err(ApiError::bad_request(format!(
            "capture_window duration exceeds maximum allowed {} seconds",
            max_interval_seconds
        )));
    }

    if payload.records.len() > 5_000 {
        return Err(ApiError {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            error: "payload_too_large".to_string(),
            message: "maximum records per batch is 5000".to_string(),
        });
    }

    if payload.records.is_empty()
        && payload.session_events.is_empty()
        && payload.diagnostics.is_empty()
    {
        return Err(ApiError::bad_request(
            "records can only be empty when session_events or diagnostics are present",
        ));
    }

    if let Some(client) = &payload.client {
        if let Some(platform) = &client.platform {
            if platform.to_lowercase() != "ios" {
                return Err(ApiError::bad_request("client.platform must be ios for MVP"));
            }
        }
    }

    // Validate envelope timestamps before opening a write transaction.
    for event in &payload.session_events {
        if map_session_event(&event.event_type).is_none() {
            return Err(ApiError::bad_request(format!(
                "unsupported session_events.event_type: {}",
                event.event_type
            )));
        }
        if !timestamp_in_capture_window(
            &event.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "session_events.observed_at must be within capture_window",
            ));
        }
    }

    for diag in &payload.diagnostics {
        if !timestamp_in_capture_window(
            &diag.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "diagnostics.observed_at must be within capture_window",
            ));
        }
    }

    for record in &payload.records {
        if !timestamp_in_capture_window(
            &record.observed_at,
            &payload.capture_window.started_at,
            &payload.capture_window.ended_at,
        ) {
            return Err(ApiError::bad_request(
                "records.observed_at must be within capture_window",
            ));
        }
        if let Some(temperature_bin) = &record.temperature_bin {
            let valid_bin = matches!(
                temperature_bin.as_str(),
                "very_cold" | "cold" | "cool" | "mild" | "hot"
            );
            if !valid_bin {
                return Err(ApiError::bad_request(
                    "records.temperature_bin must be one of very_cold,cold,cool,mild,hot",
                ));
            }
        }
    }

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
        if !state.signal_keys.contains(&record.signal_key) {
            errors.push(IngestRecordError {
                record_index: index,
                code: "unknown_signal_key".to_string(),
                message: "signal_key not present in active v0.2 registry".to_string(),
            });
            continue;
        }

        if !(record.status == "ok"
            || record.status == "stale"
            || record.status == "unavailable"
            || record.status == "not_supported"
            || record.status == "permission_denied"
            || record.status == "error")
        {
            errors.push(IngestRecordError {
                record_index: index,
                code: "invalid_status".to_string(),
                message: "invalid status enum".to_string(),
            });
            continue;
        }

        if let Some(confidence) = record.confidence {
            if !(0.0..=1.0).contains(&confidence) {
                errors.push(IngestRecordError {
                    record_index: index,
                    code: "invalid_confidence".to_string(),
                    message: "confidence must be between 0 and 1".to_string(),
                });
                continue;
            }
        }

        let value_fields_set = i64::from(record.value_number.is_some())
            + i64::from(record.value_string.is_some())
            + i64::from(record.value_bool.is_some())
            + i64::from(record.value_json.is_some());
        if value_fields_set > 1 {
            errors.push(IngestRecordError {
                record_index: index,
                code: "invalid_value_fields".to_string(),
                message: "only one of value_number/value_string/value_bool/value_json is allowed"
                    .to_string(),
            });
            continue;
        }
        if (record.status == "ok" || record.status == "stale") && value_fields_set == 0 {
            errors.push(IngestRecordError {
                record_index: index,
                code: "missing_value".to_string(),
                message: "status ok/stale requires one value field".to_string(),
            });
            continue;
        }

        let derived_temperature_bin = record.temperature_bin.clone().or_else(|| {
            match (record.signal_key.as_str(), record.value_number) {
                ("environment.ambient_temp_c", Some(temp)) => {
                    Some(derive_temperature_bin(temp).to_string())
                }
                _ => None,
            }
        });

        let session_id = record.session_id.map(|id| id.to_string());
        let value_json_text = record.value_json.as_ref().map(|v| v.to_string());

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
        .bind(session_id)
        .bind(&record.signal_key)
        .bind(record.value_number)
        .bind(&record.value_string)
        .bind(record.value_bool.map(i64::from))
        .bind(value_json_text)
        .bind(&record.unit)
        .bind(record.observed_at.to_rfc3339())
        .bind(&now)
        .bind("OBD")
        .bind(&record.source_signal)
        .bind(&record.status)
        .bind(record.confidence)
        .bind(record.freshness_ttl_seconds)
        .bind(derived_temperature_bin)
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
