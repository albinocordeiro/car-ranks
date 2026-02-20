use crate::{ApiError, DatabaseBackend};

/// Enforces backend and ranking-type constraints for the rankings endpoint.
pub(in crate::rankings) fn validate_rankings_request(
    _backend: DatabaseBackend,
    ranking_type: &str,
    temperature_bin: &str,
) -> Result<(), ApiError> {
    if !is_supported_ranking_type(ranking_type) {
        return Err(ApiError::unprocessable("unsupported ranking_type"));
    }

    if ranking_type != "ev_temperature_impact" && temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filters are currently supported only for ev_temperature_impact",
        ));
    }

    Ok(())
}

fn is_supported_ranking_type(ranking_type: &str) -> bool {
    matches!(
        ranking_type,
        "ev_temperature_impact"
            | "ev_range_efficiency"
            | "ev_charging_performance"
            | "ev_composite"
    )
}
