use chrono::{DateTime, Utc};

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
