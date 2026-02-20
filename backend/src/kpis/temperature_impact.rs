use axum::Json;

use super::temperature_impact_metrics::build_temperature_impact_metric_payload;
use super::temperature_impact_queries::{fetch_temperature_kpi_rows, fetch_vehicle_make_model};
use crate::{
    ApiError, AppState, CohortBenchmark, DatabaseBackend, KpiTempQuery, TemperatureImpactResponse,
    now_str,
};

pub(super) async fn get_kpis_temperature_impact_inner(
    state: &AppState,
    params: KpiTempQuery,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    if state.backend != DatabaseBackend::Sqlite {
        return Err(crate::errors::postgres_rollout_not_enabled(
            "/v1/kpis/temperature-impact",
        ));
    }

    let timeframe = params.timeframe.unwrap_or_else(|| "90d".to_string());
    let baseline_bin = params
        .baseline_temperature_bin
        .unwrap_or_else(|| "mild".to_string());
    let compare_bin = params
        .compare_temperature_bin
        .unwrap_or_else(|| "cold".to_string());

    let vehicle_uid = params.vehicle_uid.to_string();

    let rows = fetch_temperature_kpi_rows(
        &state.sqlite_pool,
        &vehicle_uid,
        &timeframe,
        &baseline_bin,
        &compare_bin,
    )
    .await?;

    if rows.is_empty() {
        return Err(ApiError::not_found(
            "temperature impact metrics are not available for this vehicle",
        ));
    }

    // Use make/model to define the peer cohort for percentile comparisons.
    let (make, model) = fetch_vehicle_make_model(&state.sqlite_pool, &vehicle_uid).await?;

    let payload = build_temperature_impact_metric_payload(
        &state.sqlite_pool,
        rows,
        &timeframe,
        &baseline_bin,
        &compare_bin,
        &make,
        &model,
    )
    .await?;

    Ok(Json(TemperatureImpactResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        baseline_temperature_bin: baseline_bin,
        compare_temperature_bin: compare_bin,
        metrics: payload.metrics,
        cohort_benchmark: CohortBenchmark {
            cohort_size: payload.cohort_size,
            percentiles: payload.percentiles,
        },
    }))
}
