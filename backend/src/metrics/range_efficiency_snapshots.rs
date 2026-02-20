use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};

/// Raw observation row needed by range-efficiency snapshot normalization.
pub(super) struct RangeEfficiencyObservationRow {
    pub(super) signal_key: String,
    pub(super) value_number: Option<f64>,
    pub(super) observed_at: String,
}

/// Per-timestamp signal snapshot used during range-efficiency series assembly.
#[derive(Default)]
pub(super) struct RangeEfficiencySnapshot {
    pub(super) odo: Option<f64>,
    pub(super) soc: Option<f64>,
    pub(super) speed: Option<f64>,
    pub(super) regen_power_kw: Option<f64>,
    pub(super) traction_power_kw: Option<f64>,
}

/// Normalizes raw observation rows into a timestamp-indexed snapshot map.
pub(super) fn normalize_range_efficiency_snapshots(
    obs_rows: Vec<RangeEfficiencyObservationRow>,
) -> Result<BTreeMap<DateTime<Utc>, RangeEfficiencySnapshot>> {
    let mut by_ts = BTreeMap::new();
    for row in obs_rows {
        let Some(ts) = crate::parse_ts(&row.observed_at) else {
            continue;
        };

        let entry = by_ts
            .entry(ts)
            .or_insert_with(RangeEfficiencySnapshot::default);
        match (row.signal_key.as_str(), row.value_number) {
            ("distance.odometer", Some(value)) => entry.odo = Some(value),
            ("ev.soc_pct", Some(value)) => entry.soc = Some(value),
            ("speed.vehicle", Some(value)) => entry.speed = Some(value),
            ("ev.regen_power_kw", Some(value)) => entry.regen_power_kw = Some(value),
            ("ev.traction_power_kw", Some(value)) => entry.traction_power_kw = Some(value),
            _ => {}
        }
    }

    Ok(by_ts)
}
