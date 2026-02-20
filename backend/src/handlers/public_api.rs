use anyhow::Result;
use axum::Json;
use axum::extract::{Query, State};
use serde_json::{Value, json};
#[cfg(test)]
use sqlx::PgPool;

#[cfg(test)]
use crate::KpiMetric;
use crate::{
    ApiError, AppState, GenericKpiResponse, IngestResponse, KpiQuery, KpiTempQuery, RankingsQuery,
    RankingsResponse, SamplingConfigResponse, TelemetryBatchRequest, TemperatureImpactResponse,
    now_str,
};

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

pub(crate) async fn get_kpis_me(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    // KPI reads are delegated to kpis.rs so backend-specific query logic is isolated.
    crate::kpis::get_kpis_me(State(state), Query(params)).await
}

pub(crate) async fn get_kpis_charging(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    // Keep endpoint wiring stable while charging KPI behavior evolves in one module.
    crate::kpis::get_kpis_charging(State(state), Query(params)).await
}

pub(crate) async fn get_kpis_temperature_impact(
    State(state): State<AppState>,
    Query(params): Query<KpiTempQuery>,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    // Temperature KPI aggregation and cohort percentile logic are centralized in kpis.rs.
    crate::kpis::get_kpis_temperature_impact(State(state), Query(params)).await
}

#[cfg(test)]
pub(crate) async fn fetch_latest_vehicle_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>> {
    // Postgres-specific KPI fetch path is only needed in tests for now.
    crate::kpis::fetch_latest_vehicle_kpis_postgres(
        pool,
        vehicle_uid,
        ranking_type,
        timeframe,
        temperature_bin,
    )
    .await
}

pub(crate) async fn get_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    // Ranking query construction and row materialization live in rankings.rs.
    crate::rankings::get_rankings(State(state), Query(params)).await
}
