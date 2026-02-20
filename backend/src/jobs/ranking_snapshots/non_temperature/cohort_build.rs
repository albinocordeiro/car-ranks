use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use sqlx::SqlitePool;

use super::vehicle_catalog::VehicleCatalogRow;

/// One ranked vehicle row before cohort position assignment.
#[derive(Debug)]
pub(super) struct CohortEntry {
    pub(super) vehicle_uid: String,
    pub(super) score: f64,
    pub(super) confidence_level: String,
}

/// Builds non-temperature cohort buckets for one ranking type and timeframe.
pub(super) async fn build_non_temperature_cohorts(
    pool: &SqlitePool,
    vehicle_rows: &[VehicleCatalogRow],
    ranking_type: &str,
    timeframe: &str,
) -> Result<HashMap<String, Vec<CohortEntry>>> {
    let mut cohorts: HashMap<String, Vec<CohortEntry>> = HashMap::new();

    for vehicle in vehicle_rows {
        let kpis = crate::kpis::fetch_latest_vehicle_kpis_sqlite(
            pool,
            &vehicle.vehicle_uid,
            ranking_type,
            timeframe,
            "all",
        )
        .await?;
        if kpis.is_empty() {
            continue;
        }

        let kpi_map: BTreeMap<String, f64> =
            kpis.iter().map(|k| (k.kpi_key.clone(), k.value)).collect();
        let score = crate::metrics::score_from_kpi_map(ranking_type, &kpi_map);
        let confidence_level = crate::metrics::confidence_from_kpi_metrics(&kpis).to_string();
        let cohort_key = format!(
            "bev|{}|{}|{}|{}",
            vehicle.make,
            vehicle.model,
            vehicle.trim,
            crate::year_band(vehicle.model_year)
        );

        cohorts.entry(cohort_key).or_default().push(CohortEntry {
            vehicle_uid: vehicle.vehicle_uid.clone(),
            score,
            confidence_level,
        });
    }

    Ok(cohorts)
}
