use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

use crate::{MetricCalc, metrics};

use super::super::kpi_recompute::insert_native_kpi_snapshot_postgres;
use super::health_penalty::compute_health_modifier_penalty_postgres;

const KPI_TIMEFRAMES: [&str; 3] = ["30d", "90d", "180d"];
const RANKING_TYPE: &str = "ev_composite";

/// Rebuilds composite KPI snapshots directly in Postgres.
pub(super) async fn recompute_composite_kpis_postgres(pool: &PgPool) -> Result<usize> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for native postgres composite KPI pass")?;

    sqlx::query(
        r#"
        DELETE FROM vehicle_kpi_snapshot
        WHERE ranking_type = $1
        "#,
    )
    .bind(RANKING_TYPE)
    .execute(pool)
    .await
    .context("failed to clear native postgres composite KPI snapshots")?;

    let mut rows_inserted = 0usize;
    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in native composite KPI pass")?;

        for timeframe in KPI_TIMEFRAMES {
            rows_inserted +=
                recompute_vehicle_timeframe_composite_kpis_postgres(pool, &vehicle_uid, timeframe)
                    .await?;
        }
    }

    Ok(rows_inserted)
}

async fn recompute_vehicle_timeframe_composite_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    timeframe: &str,
) -> Result<usize> {
    let range_score = fetch_latest_score_metric(
        pool,
        vehicle_uid,
        "ev_range_efficiency",
        timeframe,
        "ev_range_efficiency_score",
    )
    .await?;
    let charging_score = fetch_latest_score_metric(
        pool,
        vehicle_uid,
        "ev_charging_performance",
        timeframe,
        "charging_performance_score",
    )
    .await?;

    let Some(base_composite_score) = (match (range_score.as_ref(), charging_score.as_ref()) {
        (Some((range, _)), Some((charging, _))) => {
            Some((0.6 * range + 0.4 * charging).clamp(0.0, 100.0))
        }
        (Some((range, _)), None) => Some(range.clamp(0.0, 100.0)),
        (None, Some((charging, _))) => Some(charging.clamp(0.0, 100.0)),
        (None, None) => None,
    }) else {
        return Ok(0);
    };

    let cutoff = crate::timeframe_cutoff(timeframe)?;
    let (health_penalty, health_sample_count) =
        compute_health_modifier_penalty_postgres(pool, vehicle_uid, cutoff).await?;
    let adjusted_score = (base_composite_score - health_penalty).clamp(0.0, 100.0);

    let sample_count = range_score
        .as_ref()
        .map(|(_, sample_count)| *sample_count)
        .unwrap_or(0)
        .max(
            charging_score
                .as_ref()
                .map(|(_, sample_count)| *sample_count)
                .unwrap_or(0),
        )
        .max(health_sample_count);

    let metrics_to_persist = [
        MetricCalc {
            key: "ev_composite_base_score",
            value: base_composite_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: metrics::confidence_from_samples(sample_count),
        },
        MetricCalc {
            key: "ev_health_modifier_penalty",
            value: health_penalty,
            unit: "score_points",
            direction: "lower_is_better",
            sample_count: health_sample_count,
            confidence_level: metrics::confidence_from_samples(health_sample_count),
        },
        MetricCalc {
            key: "ev_composite_score",
            value: adjusted_score,
            unit: "score",
            direction: "higher_is_better",
            sample_count,
            confidence_level: metrics::confidence_from_samples(sample_count),
        },
    ];

    let snapshot_ts = crate::now_str();
    for metric in &metrics_to_persist {
        insert_native_kpi_snapshot_postgres(
            pool,
            RANKING_TYPE,
            vehicle_uid,
            timeframe,
            metric,
            &snapshot_ts,
        )
        .await?;
    }

    Ok(metrics_to_persist.len())
}

async fn fetch_latest_score_metric(
    pool: &PgPool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    kpi_key: &str,
) -> Result<Option<(f64, i64)>> {
    let row = sqlx::query(
        r#"
        SELECT
          kpi_value::double precision AS kpi_value,
          sample_count
        FROM vehicle_kpi_snapshot
        WHERE vehicle_uid = $1
          AND ranking_type = $2
          AND timeframe = $3
          AND temperature_bin = 'all'
          AND kpi_key = $4
        ORDER BY computed_at DESC
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(kpi_key)
    .fetch_optional(pool)
    .await
    .with_context(|| {
        format!(
            "failed to fetch latest {} score metric for native composite recompute",
            ranking_type
        )
    })?;

    let Some(row) = row else {
        return Ok(None);
    };

    let value: f64 = row
        .try_get("kpi_value")
        .with_context(|| format!("failed to parse {} value", kpi_key))?;
    let sample_count: i64 =
        row.try_get::<i32, _>("sample_count")
            .with_context(|| format!("failed to parse {} sample_count", kpi_key))? as i64;

    Ok(Some((value, sample_count)))
}
