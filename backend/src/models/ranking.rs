use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct RankingsQuery {
    pub(crate) ranking_type: String,
    pub(crate) timeframe: Option<String>,
    pub(crate) temperature_bin: Option<String>,
    pub(crate) powertrain_class: Option<String>,
    pub(crate) make: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) trim: Option<String>,
    pub(crate) year_band: Option<String>,
    pub(crate) region: Option<String>,
    pub(crate) limit: Option<i64>,
    pub(crate) offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingRow {
    pub(crate) rank: i64,
    pub(crate) vehicle_uid: Uuid,
    pub(crate) score: f64,
    pub(crate) confidence_level: String,
    pub(crate) kpis: BTreeMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingPage {
    pub(crate) limit: i64,
    pub(crate) offset: i64,
    pub(crate) has_more: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingCohort {
    pub(crate) cohort_key: String,
    pub(crate) cohort_size: i64,
    pub(crate) sample_gate_passed: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct RankingsResponse {
    pub(crate) generated_at: String,
    pub(crate) ranking_type: String,
    pub(crate) timeframe: String,
    pub(crate) temperature_bin: String,
    pub(crate) filters: BTreeMap<String, Option<String>>,
    pub(crate) cohort: RankingCohort,
    pub(crate) rows: Vec<RankingRow>,
    pub(crate) page: RankingPage,
}
