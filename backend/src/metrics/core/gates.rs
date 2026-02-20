/// Runtime gate thresholds that determine whether temperature KPIs can be
/// emitted with enough confidence.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TemperatureSampleGates {
    pub(crate) min_cold_distance_km: f64,
    pub(crate) min_mild_distance_km: f64,
    pub(crate) min_cold_charge_sessions: usize,
    pub(crate) min_mild_charge_sessions: usize,
    pub(crate) min_sensitivity_points: usize,
}

impl TemperatureSampleGates {
    /// Checks whether both cold and mild driving distance gates are satisfied.
    pub(crate) fn range_gate_passed(self, cold_distance_km: f64, mild_distance_km: f64) -> bool {
        cold_distance_km >= self.min_cold_distance_km
            && mild_distance_km >= self.min_mild_distance_km
    }

    /// Checks whether both cold and mild charging session gates are satisfied.
    pub(crate) fn charge_gate_passed(self, cold_sessions: usize, mild_sessions: usize) -> bool {
        cold_sessions >= self.min_cold_charge_sessions
            && mild_sessions >= self.min_mild_charge_sessions
    }
}

/// Reads temperature KPI gate settings from environment variables.
pub(crate) fn temperature_sample_gates() -> TemperatureSampleGates {
    let min_cold_charge_sessions =
        crate::read_positive_env("CAR_RANKS_TEMP_GATE_MIN_COLD_CHARGE_SESSIONS", 1);
    let min_mild_charge_sessions =
        crate::read_positive_env("CAR_RANKS_TEMP_GATE_MIN_MILD_CHARGE_SESSIONS", 1);
    let min_sensitivity_points =
        crate::read_positive_env("CAR_RANKS_TEMP_GATE_MIN_SENSITIVITY_POINTS", 6);

    TemperatureSampleGates {
        min_cold_distance_km: crate::read_positive_env_f64(
            "CAR_RANKS_TEMP_GATE_MIN_COLD_DISTANCE_KM",
            20.0,
        ),
        min_mild_distance_km: crate::read_positive_env_f64(
            "CAR_RANKS_TEMP_GATE_MIN_MILD_DISTANCE_KM",
            20.0,
        ),
        min_cold_charge_sessions: min_cold_charge_sessions as usize,
        min_mild_charge_sessions: min_mild_charge_sessions as usize,
        min_sensitivity_points: min_sensitivity_points as usize,
    }
}
