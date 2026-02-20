use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Query, State};

use crate::{
    ApiError, AppState, DatabaseBackend, RankingPage, RankingsQuery, RankingsResponse, now_str,
};

use self::materialization::materialize_ranking_rows;
use self::snapshot_rows::{fetch_latest_computed_at, fetch_ranking_rows};

mod kpi_details;
mod materialization;
mod snapshot_rows;

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

    let timeframe = params
        .timeframe
        .clone()
        .unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params
        .temperature_bin
        .clone()
        .unwrap_or_else(|| "all".to_string());
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    if params.ranking_type != "ev_temperature_impact" && temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filters are currently supported only for ev_temperature_impact",
        ));
    }

    let computed_at = fetch_latest_computed_at(
        &state.sqlite_pool,
        &params.ranking_type,
        &timeframe,
        &temperature_bin,
    )
    .await?;

    let rows = fetch_ranking_rows(
        &state.sqlite_pool,
        &params,
        &timeframe,
        &temperature_bin,
        &computed_at,
        limit,
        offset,
    )
    .await?;

    let (ranking_rows, cohort) = materialize_ranking_rows(
        &state.sqlite_pool,
        rows,
        &params.ranking_type,
        &timeframe,
        &temperature_bin,
    )
    .await?;

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
