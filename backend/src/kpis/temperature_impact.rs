use std::collections::BTreeMap;

use anyhow::Context;
use axum::Json;
use sqlx::Row;

use super::temperature_impact_queries::{
    fetch_cohort_kpi_values, fetch_temperature_kpi_rows, fetch_vehicle_make_model,
};
use crate::{
    ApiError, AppState, CohortBenchmark, DatabaseBackend, KpiMetric, KpiTempQuery,
    TemperatureImpactResponse, now_str, percentile_rank,
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

    let mut metrics = Vec::new();
    let mut percentiles = BTreeMap::new();
    let mut cohort_size = 0usize;

    for row in rows {
        let kpi_key: String = row.try_get("kpi_key").context("failed to parse kpi_key")?;
        let value: f64 = row
            .try_get("kpi_value")
            .context("failed to parse kpi_value")?;
        let unit = row
            .try_get::<Option<String>, _>("kpi_unit")
            .context("failed to parse kpi_unit")?
            .unwrap_or_else(|| "score".to_string());
        let direction: String = row
            .try_get("direction")
            .context("failed to parse direction")?;
        let confidence_level: String = row
            .try_get("confidence_level")
            .context("failed to parse confidence_level")?;
        let sample_count: i64 = row
            .try_get("sample_count")
            .context("failed to parse sample_count")?;

        metrics.push(KpiMetric {
            kpi_key: kpi_key.clone(),
            value,
            unit,
            direction: direction.clone(),
            confidence_level,
            sample_count,
        });

        let values = fetch_cohort_kpi_values(
            &state.sqlite_pool,
            &kpi_key,
            &timeframe,
            &baseline_bin,
            &compare_bin,
            &make,
            &model,
        )
        .await?;

        cohort_size = cohort_size.max(values.len());
        let percentile = percentile_rank(&values, value, direction == "higher_is_better");
        percentiles.insert(kpi_key, percentile);
    }

    Ok(Json(TemperatureImpactResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        baseline_temperature_bin: baseline_bin,
        compare_temperature_bin: compare_bin,
        metrics,
        cohort_benchmark: CohortBenchmark {
            cohort_size,
            percentiles,
        },
    }))
}
