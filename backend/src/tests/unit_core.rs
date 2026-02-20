use std::collections::BTreeMap;

use super::*;

#[test]
fn temperature_bin_boundaries() {
    assert_eq!(derive_temperature_bin(-10.0), "very_cold");
    assert_eq!(derive_temperature_bin(-5.0), "very_cold");
    assert_eq!(derive_temperature_bin(-4.9), "cold");
    assert_eq!(derive_temperature_bin(5.0), "cold");
    assert_eq!(derive_temperature_bin(10.0), "cool");
    assert_eq!(derive_temperature_bin(20.0), "mild");
    assert_eq!(derive_temperature_bin(30.0), "hot");
}

#[test]
fn percentile_higher_is_better() {
    let values = vec![50.0, 60.0, 70.0, 80.0];
    assert_eq!(percentile_rank(&values, 70.0, true), 75);
    assert_eq!(percentile_rank(&values, 50.0, true), 25);
}

#[test]
fn percentile_lower_is_better() {
    let values = vec![10.0, 20.0, 30.0, 40.0];
    assert_eq!(percentile_rank(&values, 20.0, false), 75);
    assert_eq!(percentile_rank(&values, 40.0, false), 25);
}

#[test]
fn locked_kpi_catalog_contains_core_composite_metric() {
    let spec = kpi_specs::lookup_kpi_spec("ev_composite", "ev_composite_score");
    assert!(spec.is_some());
}

#[test]
fn wh_per_km_from_soc_delta_works() {
    let wh_per_km = metrics::wh_per_km_from_soc_delta(5.0, 20.0, 60.0).expect("expected value");
    assert!((wh_per_km - 150.0).abs() < 0.0001);
}

#[test]
fn score_from_kpi_map_range_fallback_uses_net_efficiency() {
    let mut kpis = BTreeMap::new();
    kpis.insert("ev_estimated_practical_range".to_string(), 280.0);
    kpis.insert("ev_net_energy_efficiency".to_string(), 160.0);

    let score = metrics::score_from_kpi_map("ev_range_efficiency", &kpis);
    assert!(score > 0.0);
    assert!(score <= 100.0);
}

#[test]
fn temperature_sample_gate_checks() {
    let gates = metrics::TemperatureSampleGates {
        min_cold_distance_km: 20.0,
        min_mild_distance_km: 20.0,
        min_cold_charge_sessions: 1,
        min_mild_charge_sessions: 1,
        min_sensitivity_points: 6,
    };

    assert!(gates.range_gate_passed(20.0, 25.0));
    assert!(!gates.range_gate_passed(19.9, 25.0));
    assert!(!gates.range_gate_passed(20.0, 19.9));

    assert!(gates.charge_gate_passed(1, 1));
    assert!(!gates.charge_gate_passed(0, 1));
    assert!(!gates.charge_gate_passed(1, 0));
}
