use anyhow::Context;
use sqlx::{PgPool, Row};

use crate::ApiError;

use super::TemperatureGateEvidence;

/// Reads readiness gate evidence from PostgreSQL KPI source tables.
pub(super) async fn fetch_temperature_gate_evidence_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    cutoff_ts: &str,
) -> Result<TemperatureGateEvidence, ApiError> {
    let mut evidence = TemperatureGateEvidence::default();

    let distance_rows = sqlx::query(
        r#"
        SELECT
          CASE
            WHEN temperature_bin IN ('cold', 'very_cold') THEN 'cold'
            WHEN temperature_bin = 'mild' THEN 'mild'
            ELSE NULL
          END AS temp_group,
          MIN(value_number) AS min_odo,
          MAX(value_number) AS max_odo
        FROM vehicle_signal_observation
        WHERE vehicle_uid = $1
          AND signal_key = 'distance.odometer'
          AND value_number IS NOT NULL
          AND observed_at >= $2
          AND temperature_bin IN ('cold', 'very_cold', 'mild')
        GROUP BY temp_group
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff_ts)
    .fetch_all(pool)
    .await
    .context("failed to fetch postgres temperature distance evidence")?;

    for row in distance_rows {
        let temp_group: Option<String> = row
            .try_get("temp_group")
            .context("failed to parse postgres temperature temp_group")?;
        let min_odo: Option<f64> = row
            .try_get("min_odo")
            .context("failed to parse postgres temperature min_odo")?;
        let max_odo: Option<f64> = row
            .try_get("max_odo")
            .context("failed to parse postgres temperature max_odo")?;
        let distance = match (min_odo, max_odo) {
            (Some(min), Some(max)) if max > min => max - min,
            _ => 0.0,
        };

        match temp_group.as_deref() {
            Some("cold") => evidence.cold_distance_km = distance,
            Some("mild") => evidence.mild_distance_km = distance,
            _ => {}
        }
    }

    let session_row = sqlx::query(
        r#"
        SELECT
          SUM(CASE WHEN temperature_bin IN ('cold', 'very_cold') THEN 1 ELSE 0 END) AS cold_sessions,
          SUM(CASE WHEN temperature_bin = 'mild' THEN 1 ELSE 0 END) AS mild_sessions
        FROM vehicle_charging_session
        WHERE vehicle_uid = $1
          AND started_at >= $2
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff_ts)
    .fetch_one(pool)
    .await
    .context("failed to fetch postgres temperature charging-session evidence")?;

    let cold_sessions: Option<i64> = session_row
        .try_get("cold_sessions")
        .context("failed to parse postgres cold charging-session count")?;
    let mild_sessions: Option<i64> = session_row
        .try_get("mild_sessions")
        .context("failed to parse postgres mild charging-session count")?;

    evidence.cold_charge_sessions = cold_sessions.unwrap_or(0).max(0) as usize;
    evidence.mild_charge_sessions = mild_sessions.unwrap_or(0).max(0) as usize;

    Ok(evidence)
}
