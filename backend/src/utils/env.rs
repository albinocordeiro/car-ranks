/// Read an environment variable as a positive i64, otherwise use a default.
pub(crate) fn read_positive_env(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// Read an environment variable as a positive f64, otherwise use a default.
pub(crate) fn read_positive_env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(default)
}
