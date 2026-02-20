use anyhow::{Context, Result};
use sqlx::SqlitePool;

use self::session_metrics::{ChargingObservation, derive_session_metrics};
use self::session_rows::fetch_charging_session_windows;
use self::storage::{SessionUpsert, upsert_charging_session};

mod observation_rows;
mod session_metrics;
mod session_rows;
mod storage;

/// Reconstruct charging sessions from session events and raw signal observations.
///
/// This pass materializes the aggregate charging table so KPI jobs can compute
/// metrics from stable session-level rows instead of scanning raw observations.
pub(crate) async fn build_charging_sessions(pool: &SqlitePool) -> Result<usize> {
    let session_rows = fetch_charging_session_windows(pool).await?;

    let mut upserted = 0usize;

    for row in session_rows {
        let observations: Vec<ChargingObservation> =
            observation_rows::fetch_charging_observations_for_window(pool, &row).await?;

        let metrics = derive_session_metrics(observations, &row.started_at, &row.ended_at);
        let status = if row.ended_at.is_some() {
            "complete"
        } else {
            "partial"
        };

        upsert_charging_session(
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
        .context("failed to upsert charging session")?;

        upserted += 1;
    }

    Ok(upserted)
}
