use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

/// Vehicle identity fields needed to derive ranking cohort keys.
#[derive(Debug, Clone)]
pub(super) struct VehicleCatalogRow {
    pub(super) vehicle_uid: String,
    pub(super) make: String,
    pub(super) model: String,
    pub(super) trim: String,
    pub(super) model_year: Option<i64>,
}

/// Loads the vehicle catalog once so ranking loops can reuse it.
pub(super) async fn fetch_vehicle_catalog_rows(
    pool: &SqlitePool,
) -> Result<Vec<VehicleCatalogRow>> {
    let rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          COALESCE(make, 'unknown') AS make,
          COALESCE(model, 'unknown') AS model,
          COALESCE(trim, 'unknown') AS trim,
          model_year
        FROM vehicle
        ORDER BY vehicle_uid
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch vehicles for non-temperature rankings")?;

    let mut vehicles = Vec::with_capacity(rows.len());
    for row in rows {
        vehicles.push(VehicleCatalogRow {
            vehicle_uid: row.try_get("vehicle_uid")?,
            make: row.try_get("make")?,
            model: row.try_get("model")?,
            trim: row.try_get("trim")?,
            model_year: row.try_get("model_year")?,
        });
    }

    Ok(vehicles)
}
