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

use self::kpi_details::fetch_latest_kpi_map;
use self::snapshot_rows::{fetch_latest_computed_at, fetch_ranking_rows};

mod kpi_details;
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
        let kpis = fetch_latest_kpi_map(
            &state.sqlite_pool,
            &vehicle_uid_str,
            &params.ranking_type,
            &timeframe,
            &temperature_bin,
        )
        .await?;

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
