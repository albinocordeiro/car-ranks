use crate::{ApiError, AppState};

mod postgres;

/// Timeframe-scoped evidence used to explain temperature KPI readiness gates.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct TemperatureGateEvidence {
    pub(super) cold_distance_km: f64,
    pub(super) mild_distance_km: f64,
    pub(super) cold_charge_sessions: usize,
    pub(super) mild_charge_sessions: usize,
}

/// Fetches gate evidence from the active backend.
pub(super) async fn fetch_temperature_gate_evidence(
    state: &AppState,
    vehicle_uid: &str,
    cutoff_ts: &str,
) -> Result<TemperatureGateEvidence, ApiError> {
    postgres::fetch_temperature_gate_evidence_postgres(&state.pg_pool, vehicle_uid, cutoff_ts).await
}

/// Computes human-readable readiness gaps from current gate evidence.
pub(super) fn temperature_gate_missing_requirements(
    evidence: TemperatureGateEvidence,
) -> Vec<String> {
    let gates = crate::metrics::temperature_sample_gates();
    let mut missing = Vec::new();

    if evidence.cold_distance_km < gates.min_cold_distance_km {
        missing.push(format!(
            "cold_distance_km<{:.1}",
            gates.min_cold_distance_km
        ));
    }
    if evidence.mild_distance_km < gates.min_mild_distance_km {
        missing.push(format!(
            "mild_distance_km<{:.1}",
            gates.min_mild_distance_km
        ));
    }
    if evidence.cold_charge_sessions < gates.min_cold_charge_sessions {
        missing.push(format!(
            "cold_charging_sessions<{}",
            gates.min_cold_charge_sessions
        ));
    }
    if evidence.mild_charge_sessions < gates.min_mild_charge_sessions {
        missing.push(format!(
            "mild_charging_sessions<{}",
            gates.min_mild_charge_sessions
        ));
    }

    missing
}
