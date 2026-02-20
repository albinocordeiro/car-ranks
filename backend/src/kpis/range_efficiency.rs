use axum::Json;
use axum::extract::{Query, State};

use super::backend_router::fetch_latest_vehicle_kpis_by_backend;
use crate::{ApiError, AppState, GenericKpiResponse, KpiQuery, now_str};

/// Handles `/v1/kpis/me` by returning the latest range-efficiency KPI set.
pub(crate) async fn get_kpis_me(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params.temperature_bin.unwrap_or_else(|| "all".to_string());
    let vehicle_uid = params.vehicle_uid.to_string();

    if temperature_bin != "all" {
        return Err(ApiError::unprocessable(
            "temperature_bin filter is only supported for /v1/kpis/temperature-impact in this thin slice",
        ));
    }

    // Choose the backing query path by runtime backend so the HTTP contract stays stable.
    let kpis = fetch_latest_vehicle_kpis_by_backend(
        &state,
        &vehicle_uid,
        "ev_range_efficiency",
        &timeframe,
        &temperature_bin,
    )
    .await?;

    if kpis.is_empty() {
        return Err(ApiError::not_found(
            "range/efficiency KPIs are not available for this vehicle",
        ));
    }

    Ok(Json(GenericKpiResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        timeframe,
        temperature_bin,
        ranking_type: "ev_range_efficiency".to_string(),
        kpis,
    }))
}
