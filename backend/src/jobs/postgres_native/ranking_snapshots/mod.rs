use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

const SNAPSHOT_TIMEFRAMES: [&str; 3] = ["30d", "90d", "180d"];

/// Rebuilds charging-performance rankings directly in Postgres.
pub(super) async fn rebuild_charging_rankings_postgres(pool: &PgPool) -> Result<usize> {
    rebuild_non_temperature_rankings_postgres(pool, "ev_charging_performance").await
}

/// Rebuilds range-efficiency rankings directly in Postgres.
pub(super) async fn rebuild_range_rankings_postgres(pool: &PgPool) -> Result<usize> {
    rebuild_non_temperature_rankings_postgres(pool, "ev_range_efficiency").await
}

/// Rebuilds composite rankings directly in Postgres.
pub(super) async fn rebuild_composite_rankings_postgres(pool: &PgPool) -> Result<usize> {
    rebuild_non_temperature_rankings_postgres(pool, "ev_composite").await
}

async fn rebuild_non_temperature_rankings_postgres(
    pool: &PgPool,
    ranking_type: &str,
) -> Result<usize> {
    let mut upserted_rows = 0usize;
    let vehicles = fetch_vehicle_catalog_rows_postgres(pool).await?;

    for timeframe in SNAPSHOT_TIMEFRAMES {
        sqlx::query(
            r#"
            DELETE FROM cohort_ranking_snapshot
            WHERE ranking_type = $1
              AND timeframe = $2
            "#,
        )
        .bind(ranking_type)
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| {
            format!(
                "failed to clear native postgres ranking snapshots for {} {}",
                ranking_type, timeframe
            )
        })?;

        let ranking_snapshot_ts = crate::now_str();
        let cohorts = build_cohorts_postgres(pool, &vehicles, ranking_type, timeframe).await?;
        upserted_rows += persist_ranked_cohorts_postgres(
            pool,
            ranking_type,
            timeframe,
            &ranking_snapshot_ts,
            cohorts,
        )
        .await?;
    }

    Ok(upserted_rows)
}

#[derive(Debug, Clone)]
struct VehicleCatalogRow {
    vehicle_uid: String,
    make: String,
    model: String,
    trim: String,
    model_year: Option<i64>,
}

#[derive(Debug)]
struct CohortEntry {
    vehicle_uid: String,
    score: f64,
    confidence_level: String,
}

async fn fetch_vehicle_catalog_rows_postgres(pool: &PgPool) -> Result<Vec<VehicleCatalogRow>> {
    let rows = sqlx::query(
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
    .context("failed to fetch postgres vehicles for native rankings")?;

    let mut vehicles = Vec::with_capacity(rows.len());
    for row in rows {
        vehicles.push(VehicleCatalogRow {
            vehicle_uid: row.try_get("vehicle_uid")?,
            make: row.try_get("make")?,
            model: row.try_get("model")?,
            trim: row.try_get("trim")?,
            model_year: row.try_get("model_year")?,
        });
    }

    Ok(vehicles)
}

async fn build_cohorts_postgres(
    pool: &PgPool,
    vehicles: &[VehicleCatalogRow],
    ranking_type: &str,
    timeframe: &str,
) -> Result<HashMap<String, Vec<CohortEntry>>> {
    let mut cohorts: HashMap<String, Vec<CohortEntry>> = HashMap::new();

    for vehicle in vehicles {
        let kpis = crate::kpis::fetch_latest_vehicle_kpis_postgres(
            pool,
            &vehicle.vehicle_uid,
            ranking_type,
            timeframe,
            "all",
        )
        .await?;
        if kpis.is_empty() {
            continue;
        }

        let kpi_map: BTreeMap<String, f64> = kpis
            .iter()
            .map(|kpi| (kpi.kpi_key.clone(), kpi.value))
            .collect();
        let score = crate::metrics::score_from_kpi_map(ranking_type, &kpi_map);
        let confidence_level = crate::metrics::confidence_from_kpi_metrics(&kpis).to_string();
        let cohort_key = format!(
            "bev|{}|{}|{}|{}",
            vehicle.make,
            vehicle.model,
            vehicle.trim,
            crate::year_band(vehicle.model_year)
        );

        cohorts.entry(cohort_key).or_default().push(CohortEntry {
            vehicle_uid: vehicle.vehicle_uid.clone(),
            score,
            confidence_level,
        });
    }

    Ok(cohorts)
}

async fn persist_ranked_cohorts_postgres(
    pool: &PgPool,
    ranking_type: &str,
    timeframe: &str,
    ranking_snapshot_ts: &str,
    cohorts: HashMap<String, Vec<CohortEntry>>,
) -> Result<usize> {
    let mut upserted_rows = 0usize;

    for (cohort_key, mut entries) in cohorts {
        entries.sort_by(|a, b| crate::cmp_f64_desc(a.score, b.score));
        let cohort_size = entries.len() as i64;
        let sample_gate_passed = cohort_size >= 10;

        for (index, entry) in entries.into_iter().enumerate() {
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
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
                )
                "#,
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(ranking_type)
            .bind(timeframe)
            .bind("all")
            .bind(&cohort_key)
            .bind(cohort_size)
            .bind(if sample_gate_passed { 1_i64 } else { 0_i64 })
            .bind(&entry.vehicle_uid)
            .bind((index + 1) as i64)
            .bind(entry.score)
            .bind(&entry.confidence_level)
            .bind(ranking_snapshot_ts)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to insert native postgres ranking row for {} {}",
                    ranking_type, timeframe
                )
            })?;

            upserted_rows += 1;
        }
    }

    Ok(upserted_rows)
}
