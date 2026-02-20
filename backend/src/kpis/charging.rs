use axum::Json;
use axum::extract::{Query, State};

use super::backend_router::fetch_latest_vehicle_kpis_by_backend;
use crate::{ApiError, AppState, GenericKpiResponse, KpiQuery, now_str};

/// Handles `/v1/kpis/charging` by returning charging-performance KPIs.
pub(crate) async fn get_kpis_charging(
    State(state): State<AppState>,
    Query(params): Query<KpiQuery>,
) -> Result<Json<GenericKpiResponse>, ApiError> {
    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let temperature_bin = params.temperature_bin.unwrap_or_else(|| "all".to_string());
    let _charger_type = params.charger_type.unwrap_or_else(|| "all".to_string());
    let vehicle_uid = params.vehicle_uid.to_string();

    if temperature_bin != "all" && temperature_bin != "cold" {
        return Err(ApiError::unprocessable(
            "unsupported temperature_bin for charging KPIs in thin slice",
        ));
    }

    // Route through backend-specific readers to keep SQL dialect differences isolated.
    let kpis = fetch_latest_vehicle_kpis_by_backend(
        &state,
        &vehicle_uid,
        "ev_charging_performance",
        &timeframe,
        &temperature_bin,
    )
    .await?;

    if kpis.is_empty() {
        return Err(ApiError::not_found(
            "charging KPIs are not available for this vehicle",
        ));
    }

    Ok(Json(GenericKpiResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        timeframe,
        temperature_bin,
        ranking_type: "ev_charging_performance".to_string(),
        kpis,
    }))
}
