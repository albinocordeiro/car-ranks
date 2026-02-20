use std::cmp::Ordering;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

/// Bin ambient temperature into the canonical buckets used by KPI pipelines.
pub(crate) fn derive_temperature_bin(temp_c: f64) -> &'static str {
    if temp_c <= -5.0 {
        "very_cold"
    } else if temp_c <= 5.0 {
        "cold"
    } else if temp_c <= 15.0 {
        "cool"
    } else if temp_c <= 25.0 {
        "mild"
    } else {
        "hot"
    }
}

/// Normalize charger text labels into the small enum-like values persisted in storage.
pub(crate) fn normalize_charger_type(value: &str) -> &'static str {
    let lower = value.to_ascii_lowercase();
    if lower.contains("dc") || lower.contains("fast") {
        "dc"
    } else if lower.contains("ac") || lower.contains("level") {
        "ac"
    } else {
        "unknown"
    }
}

/// Parse RFC3339 timestamp strings as UTC datetimes.
pub(crate) fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Gate validation helper for capture-window bounded records.
pub(crate) fn timestamp_in_capture_window(
    observed_at: &DateTime<Utc>,
    started_at: &DateTime<Utc>,
    ended_at: &DateTime<Utc>,
) -> bool {
    observed_at >= started_at && observed_at <= ended_at
}

/// Return "now" in RFC3339 format for persisted metadata and API timestamps.
pub(crate) fn now_str() -> String {
    Utc::now().to_rfc3339()
}

/// Read an environment variable as a positive i64, otherwise use a default.
pub(crate) fn read_positive_env(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

/// Read an environment variable as a positive f64, otherwise use a default.
pub(crate) fn read_positive_env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| *v > 0.0)
        .unwrap_or(default)
}

/// Convert the public timeframe token into a UTC cutoff timestamp.
pub(crate) fn timeframe_cutoff(timeframe: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let cutoff = match timeframe {
        "30d" => now - Duration::days(30),
        "90d" => now - Duration::days(90),
        "180d" => now - Duration::days(180),
        "7d" => now - Duration::days(7),
        _ => return Err(anyhow::anyhow!("unsupported timeframe: {}", timeframe)),
    };
    Ok(cutoff)
}

/// Group model years into two-year bands to keep cohort cardinality stable.
pub(crate) fn year_band(model_year: Option<i64>) -> String {
    match model_year {
        Some(y) => format!("{}-{}", y, y + 2),
        None => "unknown".to_string(),
    }
}

/// Percentile rank helper shared by KPI endpoints.
pub(crate) fn percentile_rank(values: &[f64], vehicle_value: f64, higher_is_better: bool) -> i64 {
    if values.is_empty() {
        return 0;
    }

    let better_or_equal = if higher_is_better {
        values.iter().filter(|v| **v <= vehicle_value).count()
    } else {
        values.iter().filter(|v| **v >= vehicle_value).count()
    };

    ((better_or_equal as f64 / values.len() as f64) * 100.0).round() as i64
}

/// Descending sort helper that tolerates NaN by falling back to equality.
pub(crate) fn cmp_f64_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}
