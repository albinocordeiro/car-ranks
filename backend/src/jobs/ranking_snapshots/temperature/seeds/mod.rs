use anyhow::{Context, Result};
use sqlx::SqlitePool;

mod row_mapper;
mod seed_builder;

use row_mapper::map_temperature_seed_candidate;
use seed_builder::build_ranking_seed;

/// Minimal seed row used while rebuilding temperature-impact rankings.
///
/// Keeping this private to the temperature ranking path avoids coupling
/// other ranking jobs to temperature-only KPI gates.
#[derive(Debug)]
pub(super) struct VehicleRankingSeed {
    pub(super) vehicle_uid: String,
    pub(super) make: String,
    pub(super) model: String,
    pub(super) trim: String,
    pub(super) model_year: Option<i64>,
    pub(super) range_retention: Option<f64>,
    pub(super) sensitivity: Option<f64>,
    pub(super) charge_retention: Option<f64>,
    pub(super) confidence_level: String,
}

/// Fetches and filters per-vehicle KPI seeds for one timeframe.
pub(super) async fn fetch_temperature_ranking_seeds(
    pool: &SqlitePool,
    timeframe: &str,
) -> Result<Vec<VehicleRankingSeed>> {
    let rows = sqlx::query(
        r#"
        SELECT
          v.vehicle_uid,
          COALESCE(v.make, 'unknown') AS make,
          COALESCE(v.model, 'unknown') AS model,
          COALESCE(v.trim, 'unknown') AS trim,
          v.model_year,
          MAX(CASE WHEN k.kpi_key = 'cold_weather_range_retention' THEN k.kpi_value END) AS range_retention,
          MAX(CASE WHEN k.kpi_key = 'range_temperature_sensitivity_index' THEN k.kpi_value END) AS sensitivity,
          MAX(CASE WHEN k.kpi_key = 'cold_weather_charge_speed_retention' THEN k.kpi_value END) AS charge_retention
        FROM vehicle v
        LEFT JOIN vehicle_kpi_snapshot k
          ON k.vehicle_uid = v.vehicle_uid
         AND k.ranking_type = 'ev_temperature_impact'
         AND k.timeframe = ?
         AND k.temperature_bin = 'cold'
        GROUP BY v.vehicle_uid, v.make, v.model, v.trim, v.model_year
        "#,
    )
    .bind(timeframe)
    .fetch_all(pool)
    .await
    .with_context(|| format!("failed to fetch KPI seeds for timeframe {}", timeframe))?;

    let mut seeds = Vec::new();
    for row in rows {
        let candidate = map_temperature_seed_candidate(&row)?;
        if let Some(seed) = build_ranking_seed(candidate) {
            seeds.push(seed);
        }
    }

    Ok(seeds)
}
