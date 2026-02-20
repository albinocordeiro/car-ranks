use std::collections::BTreeMap;

use crate::{ApiError, DatabaseBackend, RankingsQuery};

/// Normalized pagination and scope controls for rankings reads.
pub(super) struct RankingsWindow {
    pub(super) timeframe: String,
    pub(super) temperature_bin: String,
    pub(super) limit: i64,
    pub(super) offset: i64,
}

/// Resolves default timeframe/bin values and pagination bounds.
pub(super) fn normalize_rankings_window(params: &RankingsQuery) -> RankingsWindow {
    RankingsWindow {
        timeframe: params
            .timeframe
            .clone()
            .unwrap_or_else(|| "30d".to_string()),
        temperature_bin: params
            .temperature_bin
            .clone()
            .unwrap_or_else(|| "all".to_string()),
        limit: params.limit.unwrap_or(25).clamp(1, 100),
        offset: params.offset.unwrap_or(0).max(0),
    }
}

/// Enforces backend and ranking-type constraints for the rankings endpoint.
pub(super) fn validate_rankings_request(
    backend: DatabaseBackend,
    ranking_type: &str,
    temperature_bin: &str,
) -> Result<(), ApiError> {
    if backend != DatabaseBackend::Sqlite {
        return Err(crate::errors::postgres_rollout_not_enabled("/v1/rankings"));
    }

    let supported_ranking_type = matches!(
        ranking_type,
        "ev_temperature_impact"
            | "ev_range_efficiency"
            | "ev_charging_performance"
            | "ev_composite"
    );
    if !supported_ranking_type {
        return Err(ApiError::unprocessable("unsupported ranking_type"));
    }

    if ranking_type != "ev_temperature_impact" && temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filters are currently supported only for ev_temperature_impact",
        ));
    }

    Ok(())
}

/// Materializes the response filter map from query parameters.
pub(super) fn build_rankings_filters(params: &RankingsQuery) -> BTreeMap<String, Option<String>> {
    let mut filters = BTreeMap::new();
    filters.insert(
        "powertrain_class".to_string(),
        Some(
            params
                .powertrain_class
                .clone()
                .unwrap_or_else(|| "bev".to_string()),
        ),
    );
    filters.insert("make".to_string(), params.make.clone());
    filters.insert("model".to_string(), params.model.clone());
    filters.insert("trim".to_string(), params.trim.clone());
    filters.insert("year_band".to_string(), params.year_band.clone());
    filters.insert("region".to_string(), params.region.clone());
    filters
}
