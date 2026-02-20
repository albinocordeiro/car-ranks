use anyhow::{Context, Result};
use sqlx::{PgPool, Row, SqlitePool};

use crate::KpiMetric;

pub(crate) async fn fetch_latest_vehicle_kpis_sqlite(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>> {
    let rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value, kpi_unit, direction, confidence_level, sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = ?
          AND ranking_type = ?
          AND timeframe = ?
          AND temperature_bin = ?
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch latest vehicle KPIs")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(KpiMetric {
            kpi_key: row
                .try_get("kpi_key")
                .context("failed to parse kpi_key in fetch_latest_vehicle_kpis_sqlite")?,
            value: row
                .try_get("kpi_value")
                .context("failed to parse kpi_value in fetch_latest_vehicle_kpis_sqlite")?,
            unit: row
                .try_get::<Option<String>, _>("kpi_unit")
                .context("failed to parse kpi_unit in fetch_latest_vehicle_kpis_sqlite")?
                .unwrap_or_else(|| "score".to_string()),
            direction: row
                .try_get("direction")
                .context("failed to parse direction in fetch_latest_vehicle_kpis_sqlite")?,
            confidence_level: row
                .try_get("confidence_level")
                .context("failed to parse confidence_level in fetch_latest_vehicle_kpis_sqlite")?,
            sample_count: row
                .try_get("sample_count")
                .context("failed to parse sample_count in fetch_latest_vehicle_kpis_sqlite")?,
        });
    }

    Ok(out)
}

pub(crate) async fn fetch_latest_vehicle_kpis_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<Vec<KpiMetric>> {
    let rows = sqlx::query(
        r#"
        SELECT
          kpi_key,
          kpi_value::double precision AS kpi_value,
          kpi_unit,
          direction,
          confidence_level,
          sample_count::bigint AS sample_count
        FROM vehicle_kpi_snapshot ks
        WHERE vehicle_uid = $1
          AND ranking_type = $2
          AND timeframe = $3
          AND temperature_bin = $4
          AND computed_at = (
              SELECT MAX(ks2.computed_at)
              FROM vehicle_kpi_snapshot ks2
              WHERE ks2.vehicle_uid = ks.vehicle_uid
                AND ks2.ranking_type = ks.ranking_type
                AND ks2.timeframe = ks.timeframe
                AND ks2.temperature_bin = ks.temperature_bin
                AND ks2.kpi_key = ks.kpi_key
          )
        ORDER BY kpi_key ASC
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch latest vehicle KPIs from postgres")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(KpiMetric {
            kpi_key: row
                .try_get("kpi_key")
                .context("failed to parse kpi_key in fetch_latest_vehicle_kpis_postgres")?,
            value: row
                .try_get("kpi_value")
                .context("failed to parse kpi_value in fetch_latest_vehicle_kpis_postgres")?,
            unit: row
                .try_get::<Option<String>, _>("kpi_unit")
                .context("failed to parse kpi_unit in fetch_latest_vehicle_kpis_postgres")?
                .unwrap_or_else(|| "score".to_string()),
            direction: row
                .try_get("direction")
                .context("failed to parse direction in fetch_latest_vehicle_kpis_postgres")?,
            confidence_level: row.try_get("confidence_level").context(
                "failed to parse confidence_level in fetch_latest_vehicle_kpis_postgres",
            )?,
            sample_count: row
                .try_get("sample_count")
                .context("failed to parse sample_count in fetch_latest_vehicle_kpis_postgres")?,
        });
    }

    Ok(out)
}
