use anyhow::Context;
use sqlx::{Row, SqlitePool};

use crate::ApiError;

/// Loads make/model tags used to define the percentile cohort.
pub(crate) async fn fetch_vehicle_make_model(
    pool: &SqlitePool,
    vehicle_uid: &str,
) -> Result<(String, String), ApiError> {
    let vehicle_row = sqlx::query("SELECT make, model FROM vehicle WHERE vehicle_uid = ?")
        .bind(vehicle_uid)
        .fetch_one(pool)
        .await
        .context("failed to fetch vehicle metadata")?;

    let make = vehicle_row
        .try_get::<Option<String>, _>("make")
        .context("failed to parse vehicle.make")?
        .unwrap_or_else(|| "unknown".to_string());
    let model = vehicle_row
        .try_get::<Option<String>, _>("model")
        .context("failed to parse vehicle.model")?
        .unwrap_or_else(|| "unknown".to_string());

    Ok((make, model))
}
