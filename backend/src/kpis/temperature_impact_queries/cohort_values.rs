use anyhow::Context;
use sqlx::{Row, SqlitePool};

use crate::ApiError;

/// Loads comparable cohort KPI values for percentile ranking.
pub(crate) async fn fetch_cohort_kpi_values(
    pool: &SqlitePool,
    kpi_key: &str,
    timeframe: &str,
    baseline_bin: &str,
    compare_bin: &str,
    make: &str,
    model: &str,
) -> Result<Vec<f64>, ApiError> {
    let cohort_values = sqlx::query(
        r#"
        SELECT ks.kpi_value
        FROM vehicle_kpi_snapshot ks
        JOIN vehicle v ON v.vehicle_uid = ks.vehicle_uid
        WHERE ks.kpi_key = ?
          AND ks.ranking_type = 'ev_temperature_impact'
          AND ks.timeframe = ?
          AND ks.temperature_bin = 'cold'
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
    .bind(kpi_key)
    .bind(timeframe)
    .bind(baseline_bin)
    .bind(compare_bin)
    .bind(make)
    .bind(model)
    .fetch_all(pool)
    .await
    .context("failed to fetch cohort values for percentile")?;

    Ok(cohort_values
        .into_iter()
        .filter_map(|row| row.try_get::<Option<f64>, _>("kpi_value").ok().flatten())
        .collect())
}
