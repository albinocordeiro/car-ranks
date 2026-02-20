use anyhow::Context;
use axum::Json;
use axum::extract::State;

use crate::{ApiError, AppState, DatabaseBackend, IngestResponse, TelemetryBatchRequest, now_str};

use self::idempotency::{IdempotencyOutcome, resolve_batch_idempotency};
use self::request_validation::validate_batch_payload;
use self::storage::{
    ensure_vehicle_and_batch_rows, insert_diagnostic_events, insert_session_events,
    insert_signal_observations,
};

mod idempotency;
mod record_validation;
mod request_validation;
mod storage;

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
    let ingest_id = match resolve_batch_idempotency(&mut tx, &payload, &source_upper).await? {
        IdempotencyOutcome::Fresh { ingest_id } => ingest_id,
        IdempotencyOutcome::Duplicate { response } => {
            tx.commit().await.context("failed to commit duplicate tx")?;
            return Ok(Json(response));
        }
    };

    let now = now_str();
    let vehicle_uid_str = payload.vehicle_uid.to_string();

    ensure_vehicle_and_batch_rows(
        &mut tx,
        &payload,
        &vehicle_uid_str,
        &source_account_id,
        &source_upper,
        &now,
    )
    .await?;

    let (accepted, errors) = insert_signal_observations(
        &mut tx,
        &payload,
        &vehicle_uid_str,
        &now,
        state.signal_keys.as_ref(),
    )
    .await?;

    insert_session_events(&mut tx, &payload, &vehicle_uid_str, &now).await?;
    insert_diagnostic_events(&mut tx, &payload, &vehicle_uid_str, &now).await?;

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
