use anyhow::Result;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// Raw, unfiltered seed data extracted from one ranking query row.
pub(super) struct TemperatureSeedCandidate {
    pub(super) vehicle_uid: String,
    pub(super) make: String,
    pub(super) model: String,
    pub(super) trim: String,
    pub(super) model_year: Option<i64>,
    pub(super) range_retention: Option<f64>,
    pub(super) sensitivity: Option<f64>,
    pub(super) charge_retention: Option<f64>,
}

/// Parses one SQL row into a typed candidate for downstream seed filtering.
pub(super) fn map_temperature_seed_candidate(row: &SqliteRow) -> Result<TemperatureSeedCandidate> {
    Ok(TemperatureSeedCandidate {
        vehicle_uid: row.try_get("vehicle_uid")?,
        make: row.try_get("make")?,
        model: row.try_get("model")?,
        trim: row.try_get("trim")?,
        model_year: row.try_get("model_year")?,
        range_retention: row.try_get("range_retention")?,
        sensitivity: row.try_get("sensitivity")?,
        charge_retention: row.try_get("charge_retention")?,
    })
}
