use axum::Json;
use axum::extract::{Query, State};

use crate::{ApiError, AppState, KpiTempQuery, TemperatureImpactResponse};

use self::temperature_impact::get_kpis_temperature_impact_inner;

mod backend_router;
mod charging;
mod latest_vehicle;
mod range_efficiency;
mod temperature_impact;
mod temperature_impact_metrics;
mod temperature_impact_queries;

pub(crate) use charging::get_kpis_charging;
#[allow(unused_imports)]
pub(crate) use latest_vehicle::{
    fetch_latest_vehicle_kpis_postgres, fetch_latest_vehicle_kpis_sqlite,
};
pub(crate) use range_efficiency::get_kpis_me;

pub(crate) async fn get_kpis_temperature_impact(
    State(state): State<AppState>,
    Query(params): Query<KpiTempQuery>,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    get_kpis_temperature_impact_inner(&state, params).await
}
