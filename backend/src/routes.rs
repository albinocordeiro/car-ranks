use axum::Router;
use axum::routing::{get, post};
use tower_http::trace::TraceLayer;

/// Build the complete HTTP router and attach shared middleware/state.
pub(crate) fn build_router(app_state: crate::AppState) -> Router {
    Router::new()
        .route("/health", get(crate::handlers::health))
        .route(
            "/v1/config/sampling",
            get(crate::handlers::get_config_sampling),
        )
        .route(
            "/v1/telemetry/batches",
            post(crate::handlers::post_telemetry_batches),
        )
        .route("/v1/kpis/me", get(crate::handlers::get_kpis_me))
        .route("/v1/kpis/charging", get(crate::handlers::get_kpis_charging))
        .route(
            "/v1/kpis/readiness",
            get(crate::handlers::get_kpis_readiness),
        )
        .route(
            "/v1/kpis/temperature-impact",
            get(crate::handlers::get_kpis_temperature_impact),
        )
        .route("/v1/rankings", get(crate::handlers::get_rankings))
        .route(
            "/internal/jobs/recompute-kpis",
            post(crate::handlers::post_recompute_kpis),
        )
        .route(
            "/internal/jobs/build-ranking-snapshots",
            post(crate::handlers::post_build_rankings),
        )
        .route(
            "/internal/jobs/latest",
            get(crate::handlers::get_latest_job_status),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}
