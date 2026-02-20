use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

/// Session window bounds derived from session start/stop events.
pub(super) struct ChargingSessionWindow {
    pub(super) vehicle_uid: String,
    pub(super) session_id: String,
    pub(super) started_at: String,
    pub(super) ended_at: Option<String>,
}

/// Loads charging-session windows from session event history.
pub(super) async fn fetch_charging_session_windows(
    pool: &SqlitePool,
) -> Result<Vec<ChargingSessionWindow>> {
    let rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          session_id,
          MIN(CASE WHEN event_type = 'start' THEN observed_at END) AS started_at,
          MAX(CASE WHEN event_type = 'stop' THEN observed_at END) AS ended_at
        FROM vehicle_session_event
        WHERE session_type = 'charging'
        GROUP BY vehicle_uid, session_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read charging session events")?;

    let mut windows = Vec::new();
    for row in rows {
        let vehicle_uid: String = row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in session row")?;
        let session_id: String = row
            .try_get("session_id")
            .context("invalid session_id in session row")?;
        let started_at_opt: Option<String> = row
            .try_get("started_at")
            .context("invalid started_at in session row")?;
        let ended_at: Option<String> = row
            .try_get("ended_at")
            .context("invalid ended_at in session row")?;

        let Some(started_at) = started_at_opt else {
            continue;
        };

        windows.push(ChargingSessionWindow {
            vehicle_uid,
            session_id,
            started_at,
            ended_at,
        });
    }

    Ok(windows)
}
