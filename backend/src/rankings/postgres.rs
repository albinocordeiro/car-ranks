use std::collections::BTreeMap;

use anyhow::Context;
use axum::Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::{
    ApiError, AppState, RankingCohort, RankingPage, RankingRow, RankingsQuery, RankingsResponse,
    now_str,
};

use super::request::{
    build_rankings_filters, normalize_rankings_window, validate_rankings_request,
};

/// PostgreSQL-backed `/v1/rankings` execution path.
pub(super) async fn get_rankings_postgres(
    state: &AppState,
    auth: AuthContext,
    params: RankingsQuery,
) -> Result<Json<RankingsResponse>, ApiError> {
    let pg_pool = &state.pg_pool;

    let window = normalize_rankings_window(&params);
    validate_rankings_request(&params.ranking_type, &window.temperature_bin)?;
    let user_id = auth.user_id.to_string();

    let computed_at = fetch_latest_computed_at_postgres(
        pg_pool,
        &params.ranking_type,
        &window.timeframe,
        &window.temperature_bin,
        &user_id,
    )
    .await?;

    let rows = fetch_ranking_snapshot_rows_postgres(
        pg_pool,
        &params,
        &window.timeframe,
        &window.temperature_bin,
        &computed_at,
        &user_id,
        window.limit,
        window.offset,
    )
    .await?;

    let (ranking_rows, cohort) = materialize_ranking_rows_postgres(
        pg_pool,
        rows,
        &params.ranking_type,
        &window.timeframe,
        &window.temperature_bin,
    )
    .await?;

    let has_more = ranking_rows.len() as i64 >= window.limit;
    let filters = build_rankings_filters(&params);

    Ok(Json(RankingsResponse {
        generated_at: now_str(),
        ranking_type: params.ranking_type,
        timeframe: window.timeframe,
        temperature_bin: window.temperature_bin,
        filters,
        cohort,
        rows: ranking_rows,
        page: RankingPage {
            limit: window.limit,
            offset: window.offset,
            has_more,
        },
    }))
}

async fn fetch_latest_computed_at_postgres(
    pool: &PgPool,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
    user_id: &str,
) -> Result<String, ApiError> {
    // MAX() always returns one row; when no scoped snapshots exist, the value is NULL.
    // Decode as Option<String> so no-data maps to a clean 404 instead of a decode error.
    let computed_at = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT MAX(r.computed_at)
        FROM cohort_ranking_snapshot r
        JOIN user_vehicle_access uva ON uva.vehicle_uid = r.vehicle_uid
        WHERE r.ranking_type = $1
          AND r.timeframe = $2
          AND r.temperature_bin = $3
          AND uva.user_id = $4
        "#,
    )
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("failed to fetch latest postgres ranking computed_at")?;

    computed_at.ok_or_else(|| ApiError::not_found("no ranking snapshot found for requested filter"))
}

async fn fetch_ranking_snapshot_rows_postgres(
    pool: &PgPool,
    params: &RankingsQuery,
    timeframe: &str,
    temperature_bin: &str,
    computed_at: &str,
    user_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<sqlx::postgres::PgRow>, ApiError> {
    let mut sql = String::from(
        r#"
        SELECT
          r.rank_position,
          r.vehicle_uid,
          r.score::double precision AS score,
          r.confidence_level,
          r.cohort_key,
          r.cohort_size,
          r.sample_gate_passed
        FROM cohort_ranking_snapshot r
        JOIN vehicle v ON v.vehicle_uid = r.vehicle_uid
        JOIN user_vehicle_access uva ON uva.vehicle_uid = r.vehicle_uid
        WHERE r.ranking_type = $1
          AND r.timeframe = $2
          AND r.temperature_bin = $3
          AND r.computed_at = $4
          AND uva.user_id = $5
        "#,
    );
    let next_bind_index = append_optional_filters_postgres(&mut sql, params, 6);
    let limit_bind_index = next_bind_index;
    let offset_bind_index = next_bind_index + 1;
    sql.push_str(
        format!(
            " ORDER BY r.rank_position ASC LIMIT ${limit_bind_index} OFFSET ${offset_bind_index} "
        )
        .as_str(),
    );

    let mut query = sqlx::query(&sql)
        .bind(&params.ranking_type)
        .bind(timeframe)
        .bind(temperature_bin)
        .bind(computed_at)
        .bind(user_id);

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
    query
        .fetch_all(pool)
        .await
        .context("failed to fetch postgres rankings")
        .map_err(Into::into)
}

fn append_optional_filters_postgres(
    sql: &mut String,
    params: &RankingsQuery,
    mut next_bind_index: usize,
) -> usize {
    if params.make.is_some() {
        sql.push_str(format!(" AND COALESCE(v.make, 'unknown') = ${next_bind_index} ").as_str());
        next_bind_index += 1;
    }
    if params.model.is_some() {
        sql.push_str(format!(" AND COALESCE(v.model, 'unknown') = ${next_bind_index} ").as_str());
        next_bind_index += 1;
    }
    if params.trim.is_some() {
        sql.push_str(format!(" AND COALESCE(v.trim, 'unknown') = ${next_bind_index} ").as_str());
        next_bind_index += 1;
    }
    if params.powertrain_class.is_some() {
        sql.push_str(
            format!(" AND COALESCE(v.powertrain_class, 'unknown') = ${next_bind_index} ").as_str(),
        );
        next_bind_index += 1;
    }

    next_bind_index
}

async fn materialize_ranking_rows_postgres(
    pool: &PgPool,
    rows: Vec<sqlx::postgres::PgRow>,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<(Vec<RankingRow>, RankingCohort), ApiError> {
    let mut ranking_rows = Vec::new();
    let mut cohort = RankingCohort {
        cohort_key: "unknown".to_string(),
        cohort_size: 0,
        sample_gate_passed: false,
    };

    for row in rows {
        let vehicle_uid_str: String = row
            .try_get("vehicle_uid")
            .context("failed to parse postgres ranking vehicle_uid")?;
        let vehicle_uid = Uuid::parse_str(&vehicle_uid_str)
            .context("invalid UUID stored in postgres ranking row")?;

        let kpis = fetch_latest_kpi_map_postgres(
            pool,
            &vehicle_uid_str,
            ranking_type,
            timeframe,
            temperature_bin,
        )
        .await?;

        cohort = RankingCohort {
            cohort_key: row
                .try_get("cohort_key")
                .context("failed to parse postgres cohort_key")?,
            cohort_size: row
                .try_get::<i32, _>("cohort_size")
                .context("failed to parse postgres cohort_size")? as i64,
            sample_gate_passed: row
                .try_get::<i32, _>("sample_gate_passed")
                .context("failed to parse postgres sample_gate_passed")?
                == 1,
        };

        ranking_rows.push(RankingRow {
            rank: row
                .try_get::<i32, _>("rank_position")
                .context("failed to parse postgres rank_position")? as i64,
            vehicle_uid,
            score: row
                .try_get::<f64, _>("score")
                .context("failed to parse postgres score")?,
            confidence_level: row
                .try_get("confidence_level")
                .context("failed to parse postgres confidence_level")?,
            kpis,
        });
    }

    Ok((ranking_rows, cohort))
}

async fn fetch_latest_kpi_map_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    temperature_bin: &str,
) -> Result<BTreeMap<String, f64>, ApiError> {
    let kpi_rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value::double precision AS kpi_value
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
        "#,
    )
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(temperature_bin)
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres KPI details for ranking row")?;

    let mut kpis = BTreeMap::new();
    for kpi_row in kpi_rows {
        let key: String = kpi_row
            .try_get("kpi_key")
            .context("failed to parse postgres kpi_key in ranking detail")?;
        let value: f64 = kpi_row
            .try_get("kpi_value")
            .context("failed to parse postgres kpi_value in ranking detail")?;
        kpis.insert(key, value);
    }

    Ok(kpis)
}
