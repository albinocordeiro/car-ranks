mod gates;
mod scoring;
mod stats;

pub(crate) use gates::{TemperatureSampleGates, temperature_sample_gates};
pub(crate) use scoring::{
    confidence_from_kpi_metrics, score_from_kpi_map, score_temperature_impact,
};
pub(crate) use stats::{
    confidence_from_samples, max_value, mean, median, wh_per_km_from_soc_delta,
};
