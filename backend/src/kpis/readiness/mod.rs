use axum::Json;
use axum::extract::{Query, State};

use crate::{ApiError, AppState, ReadinessQuery, ReadinessResponse, now_str};

use super::backend_router::fetch_latest_vehicle_kpis_by_backend;

use self::family_status::build_family_status;
use self::temperature_gates::{
    fetch_temperature_gate_evidence, temperature_gate_missing_requirements,
};

mod family_status;
mod temperature_gates;

const DEFAULT_TIMEFRAME: &str = "90d";
const NON_TEMPERATURE_FAMILIES: [&str; 3] = [
    "ev_range_efficiency",
    "ev_charging_performance",
    "ev_composite",
];
const TEMPERATURE_FAMILY: &str = "ev_temperature_impact";

/// Returns per-family KPI readiness signals for one vehicle/timeframe.
pub(super) async fn get_kpis_readiness(
    State(state): State<AppState>,
    Query(params): Query<ReadinessQuery>,
) -> Result<Json<ReadinessResponse>, ApiError> {
    let timeframe = params
        .timeframe
        .clone()
        .unwrap_or_else(|| DEFAULT_TIMEFRAME.to_string());
    let cutoff = crate::timeframe_cutoff(&timeframe)
        .map_err(|error| ApiError::unprocessable(error.to_string()))?;
    let cutoff_ts = cutoff.to_rfc3339();
    let vehicle_uid = params.vehicle_uid.to_string();

    let mut families = Vec::new();
    for ranking_type in NON_TEMPERATURE_FAMILIES {
        let kpis = fetch_latest_vehicle_kpis_by_backend(
            &state,
            &vehicle_uid,
            ranking_type,
            &timeframe,
            "all",
        )
        .await?;
        let missing_requirements = if kpis.is_empty() {
            vec!["kpi_snapshots_missing".to_string()]
        } else {
            Vec::new()
        };

        families.push(build_family_status(
            ranking_type,
            &kpis,
            missing_requirements,
        ));
    }

    families
        .push(build_temperature_family_status(&state, &vehicle_uid, &timeframe, &cutoff_ts).await?);

    Ok(Json(ReadinessResponse {
        vehicle_uid: params.vehicle_uid,
        generated_at: now_str(),
        timeframe,
        families,
    }))
}

async fn build_temperature_family_status(
    state: &AppState,
    vehicle_uid: &str,
    timeframe: &str,
    cutoff_ts: &str,
) -> Result<crate::ReadinessFamilyStatus, ApiError> {
    let kpis = fetch_latest_vehicle_kpis_by_backend(
        state,
        vehicle_uid,
        TEMPERATURE_FAMILY,
        timeframe,
        "cold",
    )
    .await?;
    let evidence = fetch_temperature_gate_evidence(state, vehicle_uid, cutoff_ts).await?;

    let mut missing_requirements = temperature_gate_missing_requirements(evidence);
    if kpis.is_empty() {
        missing_requirements.push("temperature_kpis_missing".to_string());
    }

    Ok(build_family_status(
        TEMPERATURE_FAMILY,
        &kpis,
        missing_requirements,
    ))
}
