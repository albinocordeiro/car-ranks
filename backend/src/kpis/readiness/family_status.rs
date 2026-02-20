use crate::{KpiMetric, ReadinessFamilyStatus};

/// Builds a normalized readiness payload row for one ranking family.
pub(super) fn build_family_status(
    ranking_type: &str,
    kpis: &[KpiMetric],
    missing_requirements: Vec<String>,
) -> ReadinessFamilyStatus {
    let confidence_level = confidence_for_family(kpis);
    let sample_count = sample_count_for_family(kpis);
    let status = readiness_status(&confidence_level, !missing_requirements.is_empty());

    ReadinessFamilyStatus {
        ranking_type: ranking_type.to_string(),
        confidence_level,
        sample_count,
        status,
        missing_requirements,
    }
}

fn confidence_for_family(kpis: &[KpiMetric]) -> String {
    if kpis.is_empty() {
        "none".to_string()
    } else {
        crate::metrics::confidence_from_kpi_metrics(kpis).to_string()
    }
}

fn sample_count_for_family(kpis: &[KpiMetric]) -> i64 {
    kpis.iter().map(|kpi| kpi.sample_count).max().unwrap_or(0)
}

fn readiness_status(confidence_level: &str, has_missing_requirements: bool) -> String {
    if has_missing_requirements || confidence_level == "none" {
        "not_ready".to_string()
    } else if confidence_level == "preview" {
        "preview".to_string()
    } else {
        "ready".to_string()
    }
}
