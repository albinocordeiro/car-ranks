use std::collections::BTreeMap;

use anyhow::Context;
use axum::Json;
use sqlx::{PgPool, Row};

use crate::{
    ApiError, AppState, CohortBenchmark, KpiMetric, KpiTempQuery, TemperatureImpactResponse,
    now_str, percentile_rank,
};

/// PostgreSQL-backed implementation for `/v1/kpis/temperature-impact`.
pub(super) async fn get_kpis_temperature_impact_postgres(
    state: &AppState,
    params: KpiTempQuery,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    let pg_pool = state
        .pg_pool
        .as_ref()
        .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;

    let timeframe = params
        .timeframe
        .clone()
        .unwrap_or_else(|| "90d".to_string());
    let baseline_bin = params
        .baseline_temperature_bin
        .clone()
        .unwrap_or_else(|| "mild".to_string());
    let compare_bin = params
        .compare_temperature_bin
        .clone()
        .unwrap_or_else(|| "cold".to_string());
    let vehicle_uid = params.vehicle_uid.to_string();

    let rows = fetch_temperature_kpi_rows_postgres(
        pg_pool,
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

    let (make, model) = fetch_vehicle_make_model_postgres(pg_pool, &vehicle_uid).await?;
    let payload = build_temperature_impact_metric_payload_postgres(
        pg_pool,
        rows,
        &timeframe,
        &baseline_bin,
        &compare_bin,
        &make,
        &model,
    )
    .await?;

    Ok(Json(TemperatureImpactResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        baseline_temperature_bin: baseline_bin,
        compare_temperature_bin: compare_bin,
        metrics: payload.metrics,
        cohort_benchmark: CohortBenchmark {
            cohort_size: payload.cohort_size,
            percentiles: payload.percentiles,
        },
    }))
}

struct TemperatureImpactMetricPayload {
    metrics: Vec<KpiMetric>,
    cohort_size: usize,
    percentiles: BTreeMap<String, i64>,
}

async fn build_temperature_impact_metric_payload_postgres(
    pool: &PgPool,
    rows: Vec<sqlx::postgres::PgRow>,
    timeframe: &str,
    baseline_bin: &str,
    compare_bin: &str,
    make: &str,
    model: &str,
) -> Result<TemperatureImpactMetricPayload, ApiError> {
    let mut metrics = Vec::with_capacity(rows.len());
    let mut percentiles = BTreeMap::new();
    let mut cohort_size = 0usize;

    for row in rows {
        let metric = map_temperature_kpi_row_postgres(&row)?;
        let values = fetch_cohort_kpi_values_postgres(
            pool,
            &metric.kpi_key,
            timeframe,
            baseline_bin,
            compare_bin,
            make,
            model,
        )
        .await?;

        cohort_size = cohort_size.max(values.len());
        let percentile = percentile_rank(
            &values,
            metric.value,
            metric.direction.as_str() == "higher_is_better",
        );
        percentiles.insert(metric.kpi_key.clone(), percentile);
        metrics.push(metric);
    }

    Ok(TemperatureImpactMetricPayload {
        metrics,
        cohort_size,
        percentiles,
    })
}

async fn fetch_temperature_kpi_rows_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    timeframe: &str,
    baseline_bin: &str,
    compare_bin: &str,
) -> Result<Vec<sqlx::postgres::PgRow>, ApiError> {
    sqlx::query(
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
          AND ranking_type = 'ev_temperature_impact'
          AND timeframe = $2
          AND temperature_bin = 'cold'
          AND baseline_temperature_bin = $3
          AND compare_temperature_bin = $4
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
    .bind(vehicle_uid)
    .bind(timeframe)
    .bind(baseline_bin)
    .bind(compare_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres temperature impact KPI rows")
    .map_err(Into::into)
}

async fn fetch_vehicle_make_model_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
) -> Result<(String, String), ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
          COALESCE(make, 'unknown') AS make,
          COALESCE(model, 'unknown') AS model
        FROM vehicle
        WHERE vehicle_uid = $1
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .fetch_optional(pool)
    .await
    .context("failed to fetch postgres vehicle make/model")?;

    let Some(row) = row else {
        return Err(ApiError::not_found(
            "vehicle not found for percentile cohorting",
        ));
    };

    let make: String = row
        .try_get("make")
        .context("failed to parse postgres make for percentile cohort")?;
    let model: String = row
        .try_get("model")
        .context("failed to parse postgres model for percentile cohort")?;

    Ok((make, model))
}

async fn fetch_cohort_kpi_values_postgres(
    pool: &PgPool,
    kpi_key: &str,
    timeframe: &str,
    baseline_bin: &str,
    compare_bin: &str,
    make: &str,
    model: &str,
) -> Result<Vec<f64>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT ks.kpi_value::double precision AS kpi_value
        FROM vehicle_kpi_snapshot ks
        JOIN vehicle v ON v.vehicle_uid = ks.vehicle_uid
        WHERE ks.kpi_key = $1
          AND ks.ranking_type = 'ev_temperature_impact'
          AND ks.timeframe = $2
          AND ks.temperature_bin = 'cold'
          AND ks.baseline_temperature_bin = $3
          AND ks.compare_temperature_bin = $4
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
          AND COALESCE(v.make, 'unknown') = $5
          AND COALESCE(v.model, 'unknown') = $6
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
    .context("failed to fetch postgres temperature cohort values")?;

    Ok(rows
        .into_iter()
        .filter_map(|row| row.try_get::<Option<f64>, _>("kpi_value").ok().flatten())
        .collect())
}

fn map_temperature_kpi_row_postgres(row: &sqlx::postgres::PgRow) -> Result<KpiMetric, ApiError> {
    let kpi_key: String = row
        .try_get("kpi_key")
        .context("failed to parse postgres temperature kpi_key")?;
    let value: f64 = row
        .try_get("kpi_value")
        .context("failed to parse postgres temperature kpi_value")?;
    let unit = row
        .try_get::<Option<String>, _>("kpi_unit")
        .context("failed to parse postgres temperature kpi_unit")?
        .unwrap_or_else(|| "score".to_string());
    let direction: String = row
        .try_get("direction")
        .context("failed to parse postgres temperature direction")?;
    let confidence_level: String = row
        .try_get("confidence_level")
        .context("failed to parse postgres temperature confidence_level")?;
    let sample_count: i64 = row
        .try_get("sample_count")
        .context("failed to parse postgres temperature sample_count")?;

    Ok(KpiMetric {
        kpi_key,
        value,
        unit,
        direction,
        confidence_level,
        sample_count,
    })
}
