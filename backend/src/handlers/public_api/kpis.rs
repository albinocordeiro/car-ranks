use anyhow::Result;
use axum::Json;
use axum::extract::{Query, State};
#[cfg(test)]
use sqlx::PgPool;

#[cfg(test)]
use crate::KpiMetric;
use crate::{
    ApiError, AppState, GenericKpiResponse, KpiQuery, KpiTempQuery, TemperatureImpactResponse,
};

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
