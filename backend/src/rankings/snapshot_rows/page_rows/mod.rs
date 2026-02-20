use anyhow::Context;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::{ApiError, RankingsQuery};

mod sql_builder;

use sql_builder::build_rankings_page_sql;

/// Fetches one page of ranking rows matching the filter set.
pub(crate) async fn fetch_ranking_rows(
    pool: &SqlitePool,
    params: &RankingsQuery,
    timeframe: &str,
    temperature_bin: &str,
    computed_at: &str,
    user_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<SqliteRow>, ApiError> {
    let sql = build_rankings_page_sql(params);

    let mut query = sqlx::query(&sql)
        .bind(&params.ranking_type)
        .bind(timeframe)
        .bind(temperature_bin)
        .bind(computed_at)
        .bind(user_id);

    // Bind values must match the order used by optional SQL filter fragments.
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
        .context("failed to fetch rankings")
        .map_err(Into::into)
}
