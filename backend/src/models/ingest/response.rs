use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(crate) struct IngestRecordError {
    pub(crate) record_index: usize,
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct IngestResponse {
    pub(crate) accepted: bool,
    pub(crate) batch_id: Uuid,
    pub(crate) ingest_id: Uuid,
    pub(crate) duplicate: bool,
    pub(crate) records_received: usize,
    pub(crate) records_accepted: usize,
    pub(crate) records_rejected: usize,
    pub(crate) errors: Vec<IngestRecordError>,
    pub(crate) next_upload_after_seconds: i64,
}
