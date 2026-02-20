use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};

/// Raw observation row needed by temperature-impact drive-series normalization.
pub(super) struct TemperatureObservationRow {
    pub(super) signal_key: String,
    pub(super) value_number: Option<f64>,
    pub(super) observed_at: String,
}

/// Per-timestamp temperature-impact snapshot used by drive-series derivation.
#[derive(Default)]
pub(super) struct TemperatureTimestampSnapshot {
    pub(super) odo: Option<f64>,
    pub(super) soc: Option<f64>,
    pub(super) temp: Option<f64>,
}

/// Normalizes observation rows into timestamp-indexed snapshots.
pub(super) fn normalize_temperature_impact_snapshots(
    obs_rows: Vec<TemperatureObservationRow>,
) -> Result<BTreeMap<DateTime<Utc>, TemperatureTimestampSnapshot>> {
    let mut by_ts = BTreeMap::new();
    for row in obs_rows {
        let Some(ts) = crate::parse_ts(&row.observed_at) else {
            continue;
        };

        let snapshot = by_ts
            .entry(ts)
            .or_insert_with(TemperatureTimestampSnapshot::default);
        match (row.signal_key.as_str(), row.value_number) {
            ("distance.odometer", Some(value)) => snapshot.odo = Some(value),
            ("ev.soc_pct", Some(value)) => snapshot.soc = Some(value),
            ("environment.ambient_temp_c", Some(value)) => snapshot.temp = Some(value),
            _ => {}
        }
    }

    Ok(by_ts)
}
