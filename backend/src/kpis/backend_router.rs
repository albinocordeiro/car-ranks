use crate::{ApiError, AppState, KpiMetric};

use super::latest_vehicle::fetch_latest_vehicle_kpis_postgres;

/// Reads latest KPI snapshots from Postgres for one vehicle/family/timeframe.
pub(super) async fn fetch_latest_vehicle_kpis_by_backend(
    state: &AppState,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>, ApiError> {
    fetch_latest_vehicle_kpis_postgres(
        &state.pg_pool,
        vehicle_uid,
        ranking_type,
        timeframe,
        temperature_bin,
    )
    .await
    .map_err(Into::into)
}
