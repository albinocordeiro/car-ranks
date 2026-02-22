use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct SessionEventInput {
    pub(crate) event_type: String,
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) session_id: Uuid,
    #[serde(default)]
    pub(crate) raw_payload_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DiagnosticInput {
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) mil_on: Option<bool>,
    pub(crate) dtcs_active: Option<Vec<String>>,
}
