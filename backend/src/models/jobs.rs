use serde::Serialize;

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
