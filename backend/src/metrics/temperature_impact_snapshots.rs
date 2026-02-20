use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// Per-timestamp temperature-impact snapshot used by drive-series derivation.
#[derive(Default)]
pub(super) struct TemperatureTimestampSnapshot {
    pub(super) odo: Option<f64>,
    pub(super) soc: Option<f64>,
    pub(super) temp: Option<f64>,
}

/// Normalizes observation rows into timestamp-indexed snapshots.
pub(super) fn normalize_temperature_impact_snapshots(
    obs_rows: Vec<SqliteRow>,
) -> Result<BTreeMap<DateTime<Utc>, TemperatureTimestampSnapshot>> {
    let mut by_ts = BTreeMap::new();
    for row in obs_rows {
        let signal_key: String = row.try_get("signal_key")?;
        let value: Option<f64> = row.try_get("value_number")?;
        let observed_at: String = row.try_get("observed_at")?;
        let Some(ts) = crate::parse_ts(&observed_at) else {
            continue;
        };

        let snapshot = by_ts
            .entry(ts)
            .or_insert_with(TemperatureTimestampSnapshot::default);
        match (signal_key.as_str(), value) {
            ("distance.odometer", Some(value)) => snapshot.odo = Some(value),
            ("ev.soc_pct", Some(value)) => snapshot.soc = Some(value),
            ("environment.ambient_temp_c", Some(value)) => snapshot.temp = Some(value),
            _ => {}
        }
    }

    Ok(by_ts)
}
