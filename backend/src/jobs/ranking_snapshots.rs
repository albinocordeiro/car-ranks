use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Minimal seed row used while rebuilding temperature-impact rankings.
/// Keeping this local to the ranking module avoids leaking ranking internals
/// into the top-level job orchestration entrypoints.
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
pub(crate) async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
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
                    sqlx::query(
                        r#"
                        INSERT INTO cohort_ranking_snapshot (
                            ranking_snapshot_id,
                            ranking_type,
                            timeframe,
                            temperature_bin,
                            cohort_key,
                            cohort_size,
                            sample_gate_passed,
                            vehicle_uid,
                            rank_position,
                            score,
                            confidence_level,
                            computed_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind("ev_temperature_impact")
                    .bind(timeframe)
                    .bind(bin)
                    .bind(&cohort_key)
                    .bind(cohort_size)
                    .bind(i64::from(sample_gate_passed))
                    .bind(&seed.vehicle_uid)
                    .bind((index + 1) as i64)
                    .bind(score)
                    .bind(&seed.confidence_level)
                    .bind(&ranking_snapshot_ts)
                    .execute(pool)
                    .await
                    .context("failed to insert cohort ranking snapshot")?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}

/// Rebuild non-temperature rankings using the latest per-vehicle KPI sets.
pub(crate) async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    let vehicle_rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          COALESCE(make, 'unknown') AS make,
          COALESCE(model, 'unknown') AS model,
          COALESCE(trim, 'unknown') AS trim,
          model_year
        FROM vehicle
        ORDER BY vehicle_uid
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch vehicles for non-temperature rankings")?;

    for timeframe in ["30d", "90d", "180d"] {
        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
        ] {
            sqlx::query(
                r#"
                DELETE FROM cohort_ranking_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear ranking snapshots for {} {}",
                    ranking_type, timeframe
                )
            })?;

            let ranking_snapshot_ts = crate::now_str();
            let mut cohorts: HashMap<String, Vec<(String, f64, String, BTreeMap<String, f64>)>> =
                HashMap::new();

            for row in &vehicle_rows {
                let vehicle_uid: String = row.try_get("vehicle_uid")?;
                let make: String = row.try_get("make")?;
                let model: String = row.try_get("model")?;
                let trim: String = row.try_get("trim")?;
                let model_year: Option<i64> = row.try_get("model_year")?;

                let kpis = crate::kpis::fetch_latest_vehicle_kpis_sqlite(
                    pool,
                    &vehicle_uid,
                    ranking_type,
                    timeframe,
                    "all",
                )
                .await?;
                if kpis.is_empty() {
                    continue;
                }

                let kpi_map: BTreeMap<String, f64> =
                    kpis.iter().map(|k| (k.kpi_key.clone(), k.value)).collect();

                let score = crate::metrics::score_from_kpi_map(ranking_type, &kpi_map);
                let confidence_level =
                    crate::metrics::confidence_from_kpi_metrics(&kpis).to_string();
                let cohort_key = format!(
                    "bev|{}|{}|{}|{}",
                    make,
                    model,
                    trim,
                    crate::year_band(model_year)
                );

                cohorts.entry(cohort_key).or_default().push((
                    vehicle_uid,
                    score,
                    confidence_level,
                    kpi_map,
                ));
            }

            for (cohort_key, mut entries) in cohorts {
                entries.sort_by(|a, b| crate::cmp_f64_desc(a.1, b.1));
                let cohort_size = entries.len() as i64;
                let sample_gate_passed = cohort_size >= 10;

                for (index, (vehicle_uid, score, confidence_level, _kpis)) in
                    entries.into_iter().enumerate()
                {
                    sqlx::query(
                        r#"
                        INSERT INTO cohort_ranking_snapshot (
                            ranking_snapshot_id,
                            ranking_type,
                            timeframe,
                            temperature_bin,
                            cohort_key,
                            cohort_size,
                            sample_gate_passed,
                            vehicle_uid,
                            rank_position,
                            score,
                            confidence_level,
                            computed_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(ranking_type)
                    .bind(timeframe)
                    .bind("all")
                    .bind(&cohort_key)
                    .bind(cohort_size)
                    .bind(i64::from(sample_gate_passed))
                    .bind(vehicle_uid)
                    .bind((index + 1) as i64)
                    .bind(score)
                    .bind(confidence_level)
                    .bind(&ranking_snapshot_ts)
                    .execute(pool)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to insert non-temperature ranking row for {} {}",
                            ranking_type, timeframe
                        )
                    })?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}
