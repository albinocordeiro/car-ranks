use std::collections::HashMap;

use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

use super::persistence::insert_cohort_ranking_snapshot;

/// Minimal seed row used while rebuilding temperature-impact rankings.
/// Keeping this private to the temperature module avoids coupling other job
/// paths to temperature-specific KPI gates.
#[derive(Debug)]
struct VehicleRankingSeed {
    vehicle_uid: String,
    make: String,
    model: String,
    trim: String,
    model_year: Option<i64>,
    range_retention: Option<f64>,
    sensitivity: Option<f64>,
    charge_retention: Option<f64>,
    confidence_level: String,
}

/// Rebuild temperature-impact rankings from the gated KPI snapshots.
pub(super) async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        let ranking_snapshot_ts = crate::now_str();
        sqlx::query(
            r#"
            DELETE FROM cohort_ranking_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear rankings for timeframe {}", timeframe))?;

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
            let vehicle_uid: String = row.try_get("vehicle_uid")?;
            let make: String = row.try_get("make")?;
            let model: String = row.try_get("model")?;
            let trim: String = row.try_get("trim")?;
            let model_year: Option<i64> = row.try_get("model_year")?;
            let range_retention: Option<f64> = row.try_get("range_retention")?;
            let sensitivity: Option<f64> = row.try_get("sensitivity")?;
            let charge_retention: Option<f64> = row.try_get("charge_retention")?;

            // Temperature impact rankings require both gated retention metrics.
            if range_retention.is_none() || charge_retention.is_none() {
                continue;
            }

            let confidence_level = if sensitivity.is_some() {
                "stable"
            } else {
                "medium"
            }
            .to_string();

            seeds.push(VehicleRankingSeed {
                vehicle_uid,
                make,
                model,
                trim,
                model_year,
                range_retention,
                sensitivity,
                charge_retention,
                confidence_level,
            });
        }

        let mut cohorts: HashMap<String, Vec<(VehicleRankingSeed, f64)>> = HashMap::new();

        for seed in seeds {
            let score = crate::metrics::score_temperature_impact(
                seed.range_retention,
                seed.charge_retention,
                seed.sensitivity,
            );
            let cohort_key = format!(
                "bev|{}|{}|{}|{}",
                seed.make,
                seed.model,
                seed.trim,
                crate::year_band(seed.model_year)
            );
            cohorts.entry(cohort_key).or_default().push((seed, score));
        }

        for (cohort_key, entries) in cohorts {
            let mut entries = entries;
            entries.sort_by(|a, b| crate::cmp_f64_desc(a.1, b.1));
            let cohort_size = entries.len() as i64;
            let sample_gate_passed = cohort_size >= 10;

            for (index, (seed, score)) in entries.into_iter().enumerate() {
                for bin in ["all", "cold"] {
                    insert_cohort_ranking_snapshot(
                        pool,
                        "ev_temperature_impact",
                        timeframe,
                        bin,
                        &cohort_key,
                        cohort_size,
                        sample_gate_passed,
                        &seed.vehicle_uid,
                        (index + 1) as i64,
                        score,
                        &seed.confidence_level,
                        &ranking_snapshot_ts,
                    )
                    .await?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}
