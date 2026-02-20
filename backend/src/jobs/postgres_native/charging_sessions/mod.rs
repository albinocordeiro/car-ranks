use anyhow::{Context, Result};
use sqlx::PgPool;

use self::observation_rows::fetch_charging_observations_for_window_postgres;
use self::session_metrics::{ChargingObservation, derive_session_metrics};
use self::session_rows::fetch_charging_session_windows_postgres;
use self::storage::{SessionUpsert, upsert_charging_session_postgres};

mod observation_rows;
mod session_metrics;
mod session_rows;
mod storage;

/// Rebuilds charging-session aggregates directly inside PostgreSQL.
///
/// The pass reads raw session observations and writes materialized charging
/// sessions without any cross-database synchronization.
pub(super) async fn build_charging_sessions(pool: &PgPool) -> Result<usize> {
    let session_rows = fetch_charging_session_windows_postgres(pool).await?;
    let mut upserted = 0usize;

    for row in session_rows {
        let observations: Vec<ChargingObservation> =
            fetch_charging_observations_for_window_postgres(pool, &row).await?;

        let metrics = derive_session_metrics(observations, &row.started_at, &row.ended_at);
        let status = if row.ended_at.is_some() {
            "complete"
        } else {
            "partial"
        };

        upsert_charging_session_postgres(
            pool,
            SessionUpsert {
                vehicle_uid: &row.vehicle_uid,
                session_id: &row.session_id,
                started_at: &row.started_at,
                ended_at: row.ended_at.as_deref(),
                status,
                metrics: &metrics,
            },
        )
        .await
        .context("failed to upsert postgres charging session")?;

        upserted += 1;
    }

    Ok(upserted)
}
