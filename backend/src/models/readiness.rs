use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query params for readiness summaries.
#[derive(Debug, Deserialize)]
pub(crate) struct ReadinessQuery {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) timeframe: Option<String>,
}

/// Readiness status for one ranking family.
#[derive(Debug, Serialize)]
pub(crate) struct ReadinessFamilyStatus {
    pub(crate) ranking_type: String,
    pub(crate) confidence_level: String,
    pub(crate) sample_count: i64,
    pub(crate) status: String,
    pub(crate) missing_requirements: Vec<String>,
}

/// Response payload for `/v1/kpis/readiness`.
#[derive(Debug, Serialize)]
pub(crate) struct ReadinessResponse {
    pub(crate) vehicle_uid: Uuid,
    pub(crate) generated_at: String,
    pub(crate) timeframe: String,
    pub(crate) families: Vec<ReadinessFamilyStatus>,
}
