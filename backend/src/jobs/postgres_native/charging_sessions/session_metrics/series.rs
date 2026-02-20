use super::ChargingObservation;

/// Intermediate vectors used to compute charging-session aggregates.
pub(super) struct SessionSeries {
    pub(super) soc_series: Vec<(String, f64)>,
    pub(super) power_series: Vec<f64>,
    pub(super) ambient_temps: Vec<f64>,
    pub(super) battery_temps: Vec<f64>,
    pub(super) charger_type: String,
}

/// Buckets observations into typed vectors used by aggregate metric builders.
pub(super) fn collect_session_series(observations: Vec<ChargingObservation>) -> SessionSeries {
    let mut soc_series: Vec<(String, f64)> = Vec::new();
    let mut power_series: Vec<f64> = Vec::new();
    let mut ambient_temps = Vec::new();
    let mut battery_temps = Vec::new();
    let mut charger_type = "unknown".to_string();

    for observation in observations {
        match observation.signal_key.as_str() {
            "ev.soc_pct" => {
                if let Some(value) = observation.value_number {
                    soc_series.push((observation.observed_at, value));
                }
            }
            "ev.charge_power_kw" | "power.battery_power_kw" => {
                if let Some(value) = observation.value_number {
                    if value.is_finite() {
                        power_series.push(value.abs());
                    }
                }
            }
            "environment.ambient_temp_c" => {
                if let Some(value) = observation.value_number {
                    ambient_temps.push(value);
                }
            }
            "ev.battery_temp_c" => {
                if let Some(value) = observation.value_number {
                    battery_temps.push(value);
                }
            }
            "ev.charger_type" => {
                if let Some(value) = observation.value_string {
                    charger_type = crate::normalize_charger_type(&value).to_string();
                }
            }
            _ => {}
        }
    }

    SessionSeries {
        soc_series,
        power_series,
        ambient_temps,
        battery_temps,
        charger_type,
    }
}
