use crate::{ApiError, AppState, DatabaseBackend, KpiMetric};

use super::latest_vehicle::{fetch_latest_vehicle_kpis_postgres, fetch_latest_vehicle_kpis_sqlite};

/// Routes latest-KPI reads through the active database backend.
pub(super) async fn fetch_latest_vehicle_kpis_by_backend(
    state: &AppState,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>, ApiError> {
    match state.backend {
        DatabaseBackend::Sqlite => fetch_latest_vehicle_kpis_sqlite(
            &state.sqlite_pool,
            vehicle_uid,
            ranking_type,
            timeframe,
            temperature_bin,
        )
        .await
        .map_err(Into::into),
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            fetch_latest_vehicle_kpis_postgres(
                pg_pool,
                vehicle_uid,
                ranking_type,
                timeframe,
                temperature_bin,
            )
            .await
            .map_err(Into::into)
        }
    }
}
