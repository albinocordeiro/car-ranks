use std::collections::BTreeMap;

use anyhow::Context;
use axum::Json;
use axum::extract::{Query, State};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    ApiError, AppState, DatabaseBackend, RankingCohort, RankingPage, RankingRow, RankingsQuery,
    RankingsResponse, now_str,
};

pub(crate) async fn get_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    if state.backend != DatabaseBackend::Sqlite {
        return Err(crate::errors::postgres_rollout_not_enabled("/v1/rankings"));
    }

    let supported_ranking_type = matches!(
        params.ranking_type.as_str(),
        "ev_temperature_impact"
            | "ev_range_efficiency"
            | "ev_charging_performance"
            | "ev_composite"
    );

    if !supported_ranking_type {
        return Err(ApiError::unprocessable("unsupported ranking_type"));
    }

    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params.temperature_bin.unwrap_or_else(|| "all".to_string());
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    if params.ranking_type != "ev_temperature_impact" && temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filters are currently supported only for ev_temperature_impact",
        ));
    }

    let latest_computed = sqlx::query(
        r#"
        SELECT MAX(computed_at) AS computed_at
        FROM cohort_ranking_snapshot
        WHERE ranking_type = ?
          AND timeframe = ?
          AND temperature_bin = ?
        "#,
    )
    .bind(&params.ranking_type)
    .bind(&timeframe)
    .bind(&temperature_bin)
    .fetch_one(&state.sqlite_pool)
    .await
    .context("failed to query ranking snapshot timestamp")?
    .try_get::<Option<String>, _>("computed_at")
    .context("failed to parse ranking computed_at")?;

    let computed_at = latest_computed
        .ok_or_else(|| ApiError::not_found("no ranking snapshot available for this filter"))?;

    let mut sql = String::from(
        r#"
        SELECT
          r.rank_position,
          r.vehicle_uid,
          r.score,
          r.confidence_level,
          r.cohort_key,
          r.cohort_size,
          r.sample_gate_passed
        FROM cohort_ranking_snapshot r
        JOIN vehicle v ON v.vehicle_uid = r.vehicle_uid
        WHERE r.ranking_type = ?
          AND r.timeframe = ?
          AND r.temperature_bin = ?
          AND r.computed_at = ?
        "#,
    );

    if params.make.is_some() {
        sql.push_str(" AND COALESCE(v.make, 'unknown') = ? ");
    }
    if params.model.is_some() {
        sql.push_str(" AND COALESCE(v.model, 'unknown') = ? ");
    }
    if params.trim.is_some() {
        sql.push_str(" AND COALESCE(v.trim, 'unknown') = ? ");
    }
    if params.powertrain_class.is_some() {
        sql.push_str(" AND COALESCE(v.powertrain_class, 'unknown') = ? ");
    }

    sql.push_str(" ORDER BY r.rank_position ASC LIMIT ? OFFSET ? ");

    let mut query = sqlx::query(&sql)
        .bind(&params.ranking_type)
        .bind(&timeframe)
        .bind(&temperature_bin)
        .bind(&computed_at);

    if let Some(make) = &params.make {
        query = query.bind(make);
    }
    if let Some(model) = &params.model {
        query = query.bind(model);
    }
    if let Some(trim) = &params.trim {
        query = query.bind(trim);
    }
    if let Some(powertrain_class) = &params.powertrain_class {
        query = query.bind(powertrain_class);
    }

    query = query.bind(limit).bind(offset);

    let rows = query
        .fetch_all(&state.sqlite_pool)
        .await
        .context("failed to fetch rankings")?;

    let mut ranking_rows = Vec::new();
    let mut cohort = RankingCohort {
        cohort_key: "unknown".to_string(),
        cohort_size: 0,
        sample_gate_passed: false,
    };

    for row in rows {
        let vehicle_uid_str: String = row
            .try_get("vehicle_uid")
            .context("failed to parse ranking vehicle_uid")?;
        let vehicle_uid =
            Uuid::parse_str(&vehicle_uid_str).context("invalid UUID stored in ranking row")?;

        // Materialize KPI details for each row so rankings are self-explanatory to clients.
        let kpi_rows = sqlx::query(
            r#"
            SELECT kpi_key, kpi_value
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
            "#,
        )
        .bind(&vehicle_uid_str)
        .bind(&params.ranking_type)
        .bind(&timeframe)
        .bind(&temperature_bin)
        .fetch_all(&state.sqlite_pool)
        .await
        .context("failed to fetch KPI details for ranking row")?;

        let mut kpis = BTreeMap::new();
        for kpi_row in kpi_rows {
            let key: String = kpi_row
                .try_get("kpi_key")
                .context("failed to parse kpi_key in ranking detail")?;
            let value: f64 = kpi_row
                .try_get("kpi_value")
                .context("failed to parse kpi_value in ranking detail")?;
            kpis.insert(key, value);
        }

        cohort = RankingCohort {
            cohort_key: row
                .try_get("cohort_key")
                .context("failed to parse cohort_key")?,
            cohort_size: row
                .try_get("cohort_size")
                .context("failed to parse cohort_size")?,
            sample_gate_passed: row
                .try_get::<i64, _>("sample_gate_passed")
                .context("failed to parse sample_gate_passed")?
                == 1,
        };

        ranking_rows.push(RankingRow {
            rank: row
                .try_get("rank_position")
                .context("failed to parse rank_position")?,
            vehicle_uid,
            score: row.try_get("score").context("failed to parse score")?,
            confidence_level: row
                .try_get("confidence_level")
                .context("failed to parse confidence_level")?,
            kpis,
        });
    }

    let has_more = ranking_rows.len() as i64 >= limit;

    let mut filters = BTreeMap::new();
    filters.insert(
        "powertrain_class".to_string(),
        Some(params.powertrain_class.unwrap_or_else(|| "bev".to_string())),
    );
    filters.insert("make".to_string(), params.make);
    filters.insert("model".to_string(), params.model);
    filters.insert("trim".to_string(), params.trim);
    filters.insert("year_band".to_string(), params.year_band);
    filters.insert("region".to_string(), params.region);

    Ok(Json(RankingsResponse {
        generated_at: now_str(),
        ranking_type: params.ranking_type,
        timeframe,
        temperature_bin,
        filters,
        cohort,
        rows: ranking_rows,
        page: RankingPage {
            limit,
            offset,
            has_more,
        },
    }))
}
