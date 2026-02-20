use axum::Json;
use axum::extract::{Query, State};

use crate::{ApiError, AppState, RankingPage, RankingsQuery, RankingsResponse, now_str};

use self::materialization::materialize_ranking_rows;
use self::request::{build_rankings_filters, normalize_rankings_window, validate_rankings_request};
use self::snapshot_rows::{fetch_latest_computed_at, fetch_ranking_rows};

mod kpi_details;
mod materialization;
mod request;
mod snapshot_rows;

pub(crate) async fn get_rankings(
    State(state): State<AppState>,
    Query(params): Query<RankingsQuery>,
) -> Result<Json<RankingsResponse>, ApiError> {
    let window = normalize_rankings_window(&params);
    validate_rankings_request(state.backend, &params.ranking_type, &window.temperature_bin)?;

    let computed_at = fetch_latest_computed_at(
        &state.sqlite_pool,
        &params.ranking_type,
        &window.timeframe,
        &window.temperature_bin,
    )
    .await?;

    let rows = fetch_ranking_rows(
        &state.sqlite_pool,
        &params,
        &window.timeframe,
        &window.temperature_bin,
        &computed_at,
        window.limit,
        window.offset,
    )
    .await?;

    let (ranking_rows, cohort) = materialize_ranking_rows(
        &state.sqlite_pool,
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
