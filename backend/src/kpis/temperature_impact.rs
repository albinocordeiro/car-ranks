use std::collections::BTreeMap;

use anyhow::Context;
use axum::Json;
use sqlx::Row;

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
    let temperature_bin = "cold";

    let vehicle_uid = params.vehicle_uid.to_string();

    let rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value, kpi_unit, direction, confidence_level, sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = ?
          AND ranking_type = 'ev_temperature_impact'
          AND timeframe = ?
          AND temperature_bin = ?
          AND baseline_temperature_bin = ?
          AND compare_temperature_bin = ?
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.baseline_temperature_bin = ks.baseline_temperature_bin
                AND ks2.compare_temperature_bin = ks.compare_temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(&vehicle_uid)
    .bind(&timeframe)
    .bind(temperature_bin)
    .bind(&baseline_bin)
    .bind(&compare_bin)
    .fetch_all(&state.sqlite_pool)
    .await
    .context("failed to fetch KPI rows")?;

    if rows.is_empty() {
        return Err(ApiError::not_found(
            "temperature impact metrics are not available for this vehicle",
        ));
    }

    // Use make/model to define the peer cohort for percentile comparisons.
    let vehicle_row = sqlx::query("SELECT make, model FROM vehicle WHERE vehicle_uid = ?")
        .bind(&vehicle_uid)
        .fetch_one(&state.sqlite_pool)
        .await
        .context("failed to fetch vehicle metadata")?;

    let make = vehicle_row
        .try_get::<Option<String>, _>("make")
        .context("failed to parse vehicle.make")?
        .unwrap_or_else(|| "unknown".to_string());
    let model = vehicle_row
        .try_get::<Option<String>, _>("model")
        .context("failed to parse vehicle.model")?
        .unwrap_or_else(|| "unknown".to_string());

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

        let cohort_values = sqlx::query(
            r#"
            SELECT ks.kpi_value
            FROM vehicle_kpi_snapshot ks
            JOIN vehicle v ON v.vehicle_uid = ks.vehicle_uid
            WHERE ks.kpi_key = ?
              AND ks.ranking_type = 'ev_temperature_impact'
              AND ks.timeframe = ?
              AND ks.temperature_bin = ?
              AND ks.baseline_temperature_bin = ?
              AND ks.compare_temperature_bin = ?
              AND ks.computed_at = (
                  SELECT MAX(ks2.computed_at)
                  FROM vehicle_kpi_snapshot ks2
                  WHERE ks2.vehicle_uid = ks.vehicle_uid
                    AND ks2.ranking_type = ks.ranking_type
                    AND ks2.timeframe = ks.timeframe
                    AND ks2.temperature_bin = ks.temperature_bin
                    AND ks2.baseline_temperature_bin = ks.baseline_temperature_bin
                    AND ks2.compare_temperature_bin = ks.compare_temperature_bin
                    AND ks2.kpi_key = ks.kpi_key
              )
              AND COALESCE(v.make, 'unknown') = ?
              AND COALESCE(v.model, 'unknown') = ?
            "#,
        )
        .bind(&kpi_key)
        .bind(&timeframe)
        .bind(temperature_bin)
        .bind(&baseline_bin)
        .bind(&compare_bin)
        .bind(&make)
        .bind(&model)
        .fetch_all(&state.sqlite_pool)
        .await
        .context("failed to fetch cohort values for percentile")?;

        let values: Vec<f64> = cohort_values
            .into_iter()
            .filter_map(|row| row.try_get::<Option<f64>, _>("kpi_value").ok().flatten())
            .collect();

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
