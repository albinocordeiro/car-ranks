use anyhow::Result;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// Charge-power vectors segmented for charging-performance KPI scoring.
pub(super) struct ChargingPowerBuckets {
    pub(super) all_power: Vec<f64>,
    pub(super) cold_power: Vec<f64>,
    pub(super) mild_power: Vec<f64>,
}

/// Normalizes charging rows into all/cold/mild power buckets.
pub(super) fn build_charging_power_buckets(
    charge_rows: Vec<SqliteRow>,
) -> Result<ChargingPowerBuckets> {
    let mut all_power = Vec::new();
    let mut cold_power = Vec::new();
    let mut mild_power = Vec::new();

    for row in charge_rows {
        let power: Option<f64> = row.try_get("avg_charge_power_kw")?;
        let bin: Option<String> = row.try_get("temperature_bin")?;
        if let (Some(power), Some(bin)) = (power, bin) {
            if power <= 0.0 || !power.is_finite() {
                continue;
            }

            all_power.push(power);
            if bin == "cold" || bin == "very_cold" {
                cold_power.push(power);
            }
            if bin == "mild" {
                mild_power.push(power);
            }
        }
    }

    Ok(ChargingPowerBuckets {
        all_power,
        cold_power,
        mild_power,
    })
}
