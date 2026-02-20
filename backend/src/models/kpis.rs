use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct KpiTempQuery {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) timeframe: Option<String>,
    pub(crate) baseline_temperature_bin: Option<String>,
    pub(crate) compare_temperature_bin: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct KpiQuery {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) timeframe: Option<String>,
    pub(crate) temperature_bin: Option<String>,
    pub(crate) charger_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct KpiMetric {
    pub(crate) kpi_key: String,
    pub(crate) value: f64,
    pub(crate) unit: String,
    pub(crate) direction: String,
    pub(crate) confidence_level: String,
    pub(crate) sample_count: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct CohortBenchmark {
    pub(crate) cohort_size: usize,
    pub(crate) percentiles: BTreeMap<String, i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TemperatureImpactResponse {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) generated_at: String,
    pub(crate) baseline_temperature_bin: String,
    pub(crate) compare_temperature_bin: String,
    pub(crate) metrics: Vec<KpiMetric>,
    pub(crate) cohort_benchmark: CohortBenchmark,
}

#[derive(Debug, Serialize)]
pub(crate) struct GenericKpiResponse {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) generated_at: String,
    pub(crate) timeframe: String,
    pub(crate) temperature_bin: String,
    pub(crate) ranking_type: String,
    pub(crate) kpis: Vec<KpiMetric>,
}
