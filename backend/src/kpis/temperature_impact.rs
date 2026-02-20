use axum::Json;

use crate::{ApiError, AppState, KpiTempQuery, TemperatureImpactResponse};

/// Delegates temperature-impact KPI reads to the Postgres implementation.
pub(super) async fn get_kpis_temperature_impact_inner(
    state: &AppState,
    params: KpiTempQuery,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    super::temperature_impact_postgres::get_kpis_temperature_impact_postgres(state, params).await
}
