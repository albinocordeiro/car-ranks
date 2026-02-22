use anyhow::Result;
use axum::Json;
use axum::extract::{Query, State};

use crate::auth::AuthContext;
use crate::{ApiError, AppState, RawTelemetryQuery, RawTelemetryResponse};

/// Auth-gated wrapper for raw telemetry inspection reads.
pub(crate) async fn get_raw_telemetry(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<RawTelemetryQuery>,
) -> Result<Json<RawTelemetryResponse>, ApiError> {
    crate::auth::ensure_vehicle_access(&state, auth.user_id, params.vehicle_uid).await?;
    crate::raw_telemetry::get_raw_telemetry(State(state), Query(params)).await
}
