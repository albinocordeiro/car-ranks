use anyhow::Result;

use crate::MetricCalc;

/// Raw charging-session row needed for charge-retention KPI scoring.
pub(super) struct ChargingPowerSampleRow {
    pub(super) avg_charge_power_kw: Option<f64>,
    pub(super) temperature_bin: Option<String>,
}

/// Splits charging-session rows into cold and mild power buckets.
pub(super) fn split_charge_power_by_temperature_bin(
    charge_rows: Vec<ChargingPowerSampleRow>,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let mut cold_charge = Vec::new();
    let mut mild_charge = Vec::new();

    for row in charge_rows {
        if let (Some(power), Some(bin)) = (row.avg_charge_power_kw, row.temperature_bin) {
            if power <= 0.0 || !power.is_finite() {
                continue;
            }
            if bin == "cold" || bin == "very_cold" {
                cold_charge.push(power);
            }
            if bin == "mild" {
                mild_charge.push(power);
            }
        }
    }

    Ok((cold_charge, mild_charge))
}

/// Scores charge-speed retention KPI once charging samples are bucketed.
pub(super) fn score_charge_retention_metric(
    cold_charge: Vec<f64>,
    mild_charge: Vec<f64>,
    gates: super::TemperatureSampleGates,
) -> Option<MetricCalc> {
    if !gates.charge_gate_passed(cold_charge.len(), mild_charge.len()) {
        return None;
    }

    if let (Some(cold), Some(mild)) = (
        super::median(cold_charge.clone()),
        super::median(mild_charge.clone()),
    ) {
        if mild > 0.0 {
            let retention = (100.0 * cold / mild).clamp(0.0, 200.0);
            let sample_count = cold_charge.len().min(mild_charge.len()) as i64;
            return Some(build_metric(
                "cold_weather_charge_speed_retention",
                retention,
                "%",
                "higher_is_better",
                sample_count,
            ));
        }
    }

    None
}

/// Centralizes charge-retention metric construction and confidence policy.
fn build_metric(
    key: &'static str,
    value: f64,
    unit: &'static str,
    direction: &'static str,
    sample_count: i64,
) -> MetricCalc {
    MetricCalc {
        key,
        value,
        unit,
        direction,
        sample_count,
        confidence_level: super::confidence_from_samples(sample_count),
    }
}
