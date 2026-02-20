use anyhow::Context;
use axum::Json;
use axum::extract::State;

use crate::{ApiError, AppState, DatabaseBackend, IngestResponse, TelemetryBatchRequest, now_str};

use self::idempotency::{IdempotencyOutcome, resolve_batch_idempotency};
use self::request_validation::validate_batch_payload;
use self::response::build_ingest_success_response;
use self::source_context::build_source_context;
use self::storage::{
    ensure_vehicle_and_batch_rows, insert_diagnostic_events, insert_session_events,
    insert_signal_observations,
};

mod idempotency;
mod record_validation;
mod request_validation;
mod response;
mod source_context;
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
    let source_context = build_source_context(&payload, validated_envelope.source_upper);

    let mut tx = state
        .sqlite_pool
        .begin()
        .await
        .context("failed to open transaction")?;

    // Resolve idempotency before writes so replayed batches can short-circuit safely.
    let ingest_id =
        match resolve_batch_idempotency(&mut tx, &payload, &source_context.source_upper).await? {
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
        &source_context.source_account_id,
        &source_context.source_upper,
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

    Ok(Json(build_ingest_success_response(
        &payload, ingest_id, accepted, errors,
    )))
}
