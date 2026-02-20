use self::series::collect_session_series;

mod series;

/// Minimal observation projection used while deriving session aggregates.
pub(super) struct ChargingObservation {
    pub(super) signal_key: String,
    pub(super) observed_at: String,
    pub(super) value_number: Option<f64>,
    pub(super) value_string: Option<String>,
}

/// Derived charging metrics written into `vehicle_charging_session`.
pub(super) struct SessionMetrics {
    pub(super) soc_start: Option<f64>,
    pub(super) soc_end: Option<f64>,
    pub(super) soc_delta: Option<f64>,
    pub(super) energy_added_kwh: Option<f64>,
    pub(super) avg_power: Option<f64>,
    pub(super) peak_power: Option<f64>,
    pub(super) ambient_avg: Option<f64>,
    pub(super) battery_avg: Option<f64>,
    pub(super) temperature_bin: Option<String>,
    pub(super) charger_type: String,
    pub(super) sample_count: i64,
}

/// Converts raw observations from one session window into aggregate metrics.
pub(super) fn derive_session_metrics(
    observations: Vec<ChargingObservation>,
    started_at: &str,
    ended_at_opt: &Option<String>,
) -> SessionMetrics {
    let mut series = collect_session_series(observations);

    series.soc_series.sort_by(|a, b| a.0.cmp(&b.0));
    let soc_start = series.soc_series.first().map(|(_, value)| *value);
    let soc_end = series.soc_series.last().map(|(_, value)| *value);
    let soc_delta = match (soc_start, soc_end) {
        (Some(start), Some(end)) => Some(end - start),
        _ => None,
    };

    let avg_power = crate::metrics::mean(&series.power_series);
    let peak_power = crate::metrics::max_value(&series.power_series);
    let ambient_avg = crate::metrics::mean(&series.ambient_temps);
    let battery_avg = crate::metrics::mean(&series.battery_temps);

    let temperature_source = ambient_avg.or(battery_avg);
    let temperature_bin = temperature_source
        .map(crate::derive_temperature_bin)
        .map(str::to_string);

    // Session duration is computed only when both endpoints parse cleanly.
    let duration_hours = match (
        crate::parse_ts(started_at),
        ended_at_opt.as_deref().and_then(crate::parse_ts),
    ) {
        (Some(start), Some(end)) if end > start => (end - start).num_seconds() as f64 / 3600.0,
        _ => 0.0,
    };

    SessionMetrics {
        soc_start,
        soc_end,
        soc_delta,
        energy_added_kwh: avg_power.map(|power| power * duration_hours.max(0.0)),
        avg_power,
        peak_power,
        ambient_avg,
        battery_avg,
        temperature_bin,
        charger_type: series.charger_type,
        sample_count: series.power_series.len() as i64,
    }
}
