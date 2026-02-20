use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use super::session_metrics::SessionMetrics;

mod statement;

use statement::execute_session_upsert_postgres;

/// Fully-validated payload for one Postgres charging-session upsert.
pub(super) struct SessionUpsert<'a> {
    pub(super) vehicle_uid: &'a str,
    pub(super) session_id: &'a str,
    pub(super) started_at: &'a str,
    pub(super) ended_at: Option<&'a str>,
    pub(super) status: &'a str,
    pub(super) metrics: &'a SessionMetrics,
}

/// Persists one charging-session aggregate row in Postgres.
pub(super) async fn upsert_charging_session_postgres(
    pool: &PgPool,
    payload: SessionUpsert<'_>,
) -> Result<()> {
    let charging_session_id = Uuid::new_v4().to_string();
    let created_at = crate::now_str();
    let updated_at = crate::now_str();
    execute_session_upsert_postgres(
        pool,
        payload,
        &charging_session_id,
        created_at.as_str(),
        updated_at.as_str(),
    )
    .await
}
