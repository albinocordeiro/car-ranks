use axum::Json;
use axum::extract::State;

use crate::auth::AuthContext;
use crate::{ApiError, AppState, IngestResponse, TelemetryBatchRequest};

mod idempotency_envelope;
mod idempotency_response;
mod postgres;
mod record_validation;
mod request_validation;
mod response;
mod source_context;

pub(crate) const INGEST_SCHEMA_VERSION: &str = "0.2";

pub(crate) async fn post_telemetry_batches(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<TelemetryBatchRequest>,
) -> Result<Json<IngestResponse>, ApiError> {
    postgres::post_telemetry_batches_postgres(
        &state.pg_pool,
        auth,
        payload,
        state.signal_keys.as_ref(),
    )
    .await
}
