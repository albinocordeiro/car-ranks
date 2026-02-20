use anyhow::Result;
use axum::Json;
use axum::extract::{Query, State};
use serde_json::{Value, json};

use crate::{
    ApiError, AppState, IngestResponse, RankingsQuery, RankingsResponse, SamplingConfigResponse,
    TelemetryBatchRequest, now_str,
};

mod kpis;

#[cfg(test)]
pub(crate) use kpis::fetch_latest_vehicle_kpis_postgres;
pub(crate) use kpis::{get_kpis_charging, get_kpis_me, get_kpis_temperature_impact};

pub(crate) async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "car-ranks-backend",
        "timestamp": now_str()
    }))
}

pub(crate) async fn get_config_sampling() -> Json<SamplingConfigResponse> {
    crate::config::get_config_sampling().await
}

pub(crate) async fn post_telemetry_batches(
    State(state): State<AppState>,
    Json(payload): Json<TelemetryBatchRequest>,
) -> Result<Json<IngestResponse>, ApiError> {
    // Keep router-facing handlers thin; ingestion rules and persistence live in ingest.rs.
    crate::ingest::post_telemetry_batches(State(state), Json(payload)).await
}

pub(crate) async fn get_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    // Ranking query construction and row materialization live in rankings.rs.
    crate::rankings::get_rankings(State(state), Query(params)).await
}
