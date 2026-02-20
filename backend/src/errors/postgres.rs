use super::ApiError;

pub(crate) fn postgres_rollout_not_enabled(endpoint: &str) -> ApiError {
    ApiError::not_implemented(format!(
        "{} is not yet enabled for postgres runtime; currently supported: /health, /v1/config/sampling, /v1/kpis/me, /v1/kpis/charging",
        endpoint
    ))
}
