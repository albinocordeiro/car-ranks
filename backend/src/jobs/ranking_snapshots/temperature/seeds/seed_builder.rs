use super::VehicleRankingSeed;
use super::row_mapper::TemperatureSeedCandidate;

/// Converts a raw candidate row into a ranking seed if required KPIs are present.
pub(super) fn build_ranking_seed(
    candidate: TemperatureSeedCandidate,
) -> Option<VehicleRankingSeed> {
    // Temperature rankings are only valid when both retention KPIs pass
    // their upstream sampling gates and are present on the seed row.
    if candidate.range_retention.is_none() || candidate.charge_retention.is_none() {
        return None;
    }

    Some(VehicleRankingSeed {
        vehicle_uid: candidate.vehicle_uid,
        make: candidate.make,
        model: candidate.model,
        trim: candidate.trim,
        model_year: candidate.model_year,
        range_retention: candidate.range_retention,
        sensitivity: candidate.sensitivity,
        charge_retention: candidate.charge_retention,
        confidence_level: infer_seed_confidence(candidate.sensitivity),
    })
}

fn infer_seed_confidence(sensitivity: Option<f64>) -> String {
    if sensitivity.is_some() {
        "stable".to_string()
    } else {
        "medium".to_string()
    }
}
