use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};

use super::session_metrics::ChargingObservation;
use super::session_rows::ChargingSessionWindow;

/// Loads and decodes signal observations for one charging-session window.
pub(super) async fn fetch_charging_observations_for_window(
    pool: &SqlitePool,
    window: &ChargingSessionWindow,
) -> Result<Vec<ChargingObservation>> {
    // Partial sessions use the current timestamp as an upper query bound so
    // we can include all observations captured so far.
    let ended_at_query = window.ended_at.clone().unwrap_or_else(crate::now_str);

    sqlx::query(
        r#"
        SELECT signal_key, value_number, value_string, observed_at
        FROM vehicle_signal_observation
        WHERE vehicle_uid = ?
          AND observed_at >= ?
          AND observed_at <= ?
        ORDER BY observed_at ASC
        "#,
    )
    .bind(&window.vehicle_uid)
    .bind(&window.started_at)
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
    .context("failed to decode charging session observations")
}
