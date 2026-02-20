use std::collections::HashSet;

use axum::Json;
use sqlx::PgPool;

use crate::auth::AuthContext;
use crate::{ApiError, IngestResponse, TelemetryBatchRequest, now_str};

use super::request_validation::validate_batch_payload;
use super::response::build_ingest_success_response;
use super::source_context::build_source_context;

mod idempotency;
mod persistence;

use idempotency::{PostgresIdempotencyOutcome, resolve_batch_idempotency_postgres};
use persistence::persist_validated_batch_postgres;

/// PostgreSQL ingest execution path.
pub(super) async fn post_telemetry_batches_postgres(
    pg_pool: &PgPool,
    auth: AuthContext,
    payload: TelemetryBatchRequest,
    signal_keys: &HashSet<String>,
) -> Result<Json<IngestResponse>, ApiError> {
    let validated_envelope = validate_batch_payload(&payload)?;
    let source_context = build_source_context(&payload, validated_envelope.source_upper);

    let mut tx = pg_pool.begin().await.map_err(|error| {
        ApiError::internal(format!("failed to open postgres transaction: {error}"))
    })?;

    // Resolve idempotency before writes so replayed batches can short-circuit safely.
    let ingest_id =
        match resolve_batch_idempotency_postgres(&mut tx, &payload, &source_context.source_upper)
            .await?
        {
            PostgresIdempotencyOutcome::Fresh { ingest_id } => ingest_id,
            PostgresIdempotencyOutcome::Duplicate { response } => {
                tx.commit().await.map_err(|error| {
                    ApiError::internal(format!("failed to commit duplicate postgres tx: {error}"))
                })?;
                return Ok(Json(response));
            }
        };

    let now = now_str();
    let auth_user_id = auth.user_id.to_string();
    let (accepted, errors) = persist_validated_batch_postgres(
        &mut tx,
        &payload,
        &source_context,
        &auth_user_id,
        &now,
        signal_keys,
    )
    .await?;

    tx.commit().await.map_err(|error| {
        ApiError::internal(format!("failed to commit postgres ingest tx: {error}"))
    })?;

    Ok(Json(build_ingest_success_response(
        &payload, ingest_id, accepted, errors,
    )))
}
