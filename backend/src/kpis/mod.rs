use axum::Json;
use axum::extract::{Query, State};

use crate::{
    ApiError, AppState, DatabaseBackend, GenericKpiResponse, KpiQuery, KpiTempQuery,
    TemperatureImpactResponse, now_str,
};

use self::temperature_impact::get_kpis_temperature_impact_inner;

mod latest_vehicle;
mod temperature_impact;
mod temperature_impact_queries;

pub(crate) use latest_vehicle::{
    fetch_latest_vehicle_kpis_postgres, fetch_latest_vehicle_kpis_sqlite,
};

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
    let kpis = match state.backend {
        DatabaseBackend::Sqlite => {
            fetch_latest_vehicle_kpis_sqlite(
                &state.sqlite_pool,
                &vehicle_uid,
                "ev_range_efficiency",
                &timeframe,
                &temperature_bin,
            )
            .await?
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            fetch_latest_vehicle_kpis_postgres(
                pg_pool,
                &vehicle_uid,
                "ev_range_efficiency",
                &timeframe,
                &temperature_bin,
            )
            .await?
        }
    };

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
    let kpis = match state.backend {
        DatabaseBackend::Sqlite => {
            fetch_latest_vehicle_kpis_sqlite(
                &state.sqlite_pool,
                &vehicle_uid,
                "ev_charging_performance",
                &timeframe,
                &temperature_bin,
            )
            .await?
        }
        DatabaseBackend::Postgres => {
            let pg_pool = state
                .pg_pool
                .as_ref()
                .ok_or_else(|| ApiError::internal("postgres pool is not configured"))?;
            fetch_latest_vehicle_kpis_postgres(
                pg_pool,
                &vehicle_uid,
                "ev_charging_performance",
                &timeframe,
                &temperature_bin,
            )
            .await?
        }
    };

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

pub(crate) async fn get_kpis_temperature_impact(
    State(state): State<AppState>,
    Query(params): Query<KpiTempQuery>,
) -> Result<Json<TemperatureImpactResponse>, ApiError> {
    get_kpis_temperature_impact_inner(&state, params).await
}
