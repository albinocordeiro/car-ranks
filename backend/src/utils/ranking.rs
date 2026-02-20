use std::cmp::Ordering;

/// Group model years into two-year bands to keep cohort cardinality stable.
pub(crate) fn year_band(model_year: Option<i64>) -> String {
    match model_year {
        Some(year) => format!("{}-{}", year, year + 2),
        None => "unknown".to_string(),
    }
}

/// Percentile rank helper shared by KPI endpoints.
pub(crate) fn percentile_rank(values: &[f64], vehicle_value: f64, higher_is_better: bool) -> i64 {
    if values.is_empty() {
        return 0;
    }

    let better_or_equal = if higher_is_better {
        values
            .iter()
            .filter(|value| **value <= vehicle_value)
            .count()
    } else {
        values
            .iter()
            .filter(|value| **value >= vehicle_value)
            .count()
    };

    ((better_or_equal as f64 / values.len() as f64) * 100.0).round() as i64
}

/// Descending sort helper that tolerates NaN by falling back to equality.
pub(crate) fn cmp_f64_desc(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
}
