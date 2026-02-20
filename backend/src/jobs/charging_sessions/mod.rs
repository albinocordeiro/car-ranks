use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

use self::session_metrics::{ChargingObservation, derive_session_metrics};
use self::storage::{SessionUpsert, upsert_charging_session};

mod session_metrics;
mod storage;

/// Reconstruct charging sessions from session events and raw signal observations.
///
/// This pass materializes the aggregate charging table so KPI jobs can compute
/// metrics from stable session-level rows instead of scanning raw observations.
pub(crate) async fn build_charging_sessions(pool: &SqlitePool) -> Result<usize> {
    let session_rows = sqlx::query(
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

    let mut upserted = 0usize;

    for row in session_rows {
        let vehicle_uid: String = row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in session row")?;
        let session_id: String = row
            .try_get("session_id")
            .context("invalid session_id in session row")?;
        let started_at_opt: Option<String> = row
            .try_get("started_at")
            .context("invalid started_at in session row")?;
        let ended_at_opt: Option<String> = row
            .try_get("ended_at")
            .context("invalid ended_at in session row")?;

        let Some(started_at) = started_at_opt else {
            continue;
        };

        // Partial sessions use the current timestamp as an upper query bound so
        // we can include all observations captured so far.
        let ended_at_query = ended_at_opt.clone().unwrap_or_else(crate::now_str);

        let observations = sqlx::query(
            r#"
            SELECT signal_key, value_number, value_string, observed_at
            FROM vehicle_signal_observation
            WHERE vehicle_uid = ?
              AND observed_at >= ?
              AND observed_at <= ?
            ORDER BY observed_at ASC
            "#,
        )
        .bind(&vehicle_uid)
        .bind(&started_at)
        .bind(&ended_at_query)
        .fetch_all(pool)
        .await
        .context("failed to fetch observations for charging session")?
        .into_iter()
        .map(|observation_row| {
            Ok(ChargingObservation {
                signal_key: observation_row.try_get("signal_key")?,
                observed_at: observation_row.try_get("observed_at")?,
                value_number: observation_row.try_get("value_number")?,
                value_string: observation_row.try_get("value_string")?,
            })
        })
        .collect::<std::result::Result<Vec<_>, sqlx::Error>>()
        .context("failed to decode charging session observations")?;

        let metrics = derive_session_metrics(observations, &started_at, &ended_at_opt);
        let status = if ended_at_opt.is_some() {
            "complete"
        } else {
            "partial"
        };

        upsert_charging_session(
            pool,
            SessionUpsert {
                vehicle_uid: &vehicle_uid,
                session_id: &session_id,
                started_at: &started_at,
                ended_at: ended_at_opt.as_deref(),
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
