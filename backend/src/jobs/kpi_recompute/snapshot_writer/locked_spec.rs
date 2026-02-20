use anyhow::Result;

use crate::MetricCalc;

/// Locked KPI metadata used for provenance logging before persistence.
pub(super) struct LockedKpiSnapshotSpec {
    pub(super) formula: &'static str,
    pub(super) required_signals: &'static [&'static str],
    pub(super) optional_signals: &'static [&'static str],
}

/// Validates one KPI snapshot against lock metadata and basic sample invariants.
pub(super) fn validate_locked_kpi_snapshot(
    ranking_type: &str,
    metric: &MetricCalc,
) -> Result<LockedKpiSnapshotSpec> {
    let Some((formula, required_signals, optional_signals)) =
        crate::kpi_specs::locked_kpi_spec_details(ranking_type, metric.key)
    else {
        return Err(anyhow::anyhow!(
            "kpi_key {} is not locked for ranking_type {}",
            metric.key,
            ranking_type
        ));
    };
    if metric.sample_count < 0 {
        return Err(anyhow::anyhow!(
            "kpi_key {} has invalid negative sample_count {}",
            metric.key,
            metric.sample_count
        ));
    }

    Ok(LockedKpiSnapshotSpec {
        formula,
        required_signals,
        optional_signals,
    })
}
