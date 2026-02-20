mod catalog;

pub(crate) use catalog::LOCKED_KPI_SPECS;

/// Canonical locked KPI specification metadata.
///
/// This catalog is intentionally centralized so every runtime path (jobs, APIs,
/// logging, tests) reads the same definitions for formulas and signal
/// dependencies.
#[derive(Debug)]
pub(crate) struct LockedKpiSpec {
    pub(crate) ranking_type: &'static str,
    pub(crate) kpi_key: &'static str,
    pub(crate) formula: &'static str,
    pub(crate) required_signals: &'static [&'static str],
    pub(crate) optional_signals: &'static [&'static str],
}

pub(crate) fn lookup_kpi_spec(ranking_type: &str, kpi_key: &str) -> Option<&'static LockedKpiSpec> {
    LOCKED_KPI_SPECS
        .iter()
        .find(|spec| spec.ranking_type == ranking_type && spec.kpi_key == kpi_key)
}

pub(crate) fn locked_kpi_spec_details(
    ranking_type: &str,
    kpi_key: &str,
) -> Option<(
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
)> {
    lookup_kpi_spec(ranking_type, kpi_key)
        .map(|spec| (spec.formula, spec.required_signals, spec.optional_signals))
}

pub(crate) fn locked_kpi_catalog_len() -> usize {
    LOCKED_KPI_SPECS.len()
}
