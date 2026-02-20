use serde::{Deserialize, Serialize};

/// Response body for the internal KPI rebuild job endpoint.
#[derive(Debug, Serialize)]
pub(crate) struct JobResponse {
    pub(crate) ok: bool,
    pub(crate) job_id: String,
    pub(crate) charging_sessions_upserted: usize,
    pub(crate) kpi_rows_upserted: usize,
    pub(crate) ranking_rows_upserted: usize,
    pub(crate) recomputed_vehicles: usize,
}

/// Query params for internal job status lookups.
#[derive(Debug, Deserialize)]
pub(crate) struct JobStatusQuery {
    pub(crate) job_kind: Option<String>,
}

/// Latest persisted status for one internal job family.
#[derive(Debug, Serialize)]
pub(crate) struct JobRunStatusResponse {
    pub(crate) job_run_id: String,
    pub(crate) job_kind: String,
    pub(crate) backend: String,
    pub(crate) status: String,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) response_job_id: Option<String>,
    pub(crate) charging_sessions_upserted: Option<i64>,
    pub(crate) kpi_rows_upserted: Option<i64>,
    pub(crate) ranking_rows_upserted: Option<i64>,
    pub(crate) recomputed_vehicles: Option<i64>,
}
