/// Internal metric calculation payload used between KPI computation layers and
/// snapshot persistence.
#[derive(Debug)]
pub(crate) struct MetricCalc {
    pub(crate) key: &'static str,
    pub(crate) value: f64,
    pub(crate) unit: &'static str,
    pub(crate) direction: &'static str,
    pub(crate) sample_count: i64,
    pub(crate) confidence_level: &'static str,
}
