use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

use super::persistence::insert_cohort_ranking_snapshot;

/// Rebuild non-temperature rankings using the latest per-vehicle KPI sets.
pub(super) async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
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
                    insert_cohort_ranking_snapshot(
                        pool,
                        ranking_type,
                        timeframe,
                        "all",
                        &cohort_key,
                        cohort_size,
                        sample_gate_passed,
                        &vehicle_uid,
                        (index + 1) as i64,
                        score,
                        &confidence_level,
                        &ranking_snapshot_ts,
                    )
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
