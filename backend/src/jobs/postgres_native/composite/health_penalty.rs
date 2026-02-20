use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

/// Computes health penalty points from active DTC and MIL status in Postgres.
pub(super) async fn compute_health_modifier_penalty_postgres(
    pool: &PgPool,
    vehicle_uid: &str,
    cutoff: DateTime<Utc>,
) -> Result<(f64, i64)> {
    let dtc_row = sqlx::query(
        r#"
        SELECT COUNT(DISTINCT code) AS dtc_count
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = $1
          AND observed_at >= $2
          AND event_type = 'DTC_ACTIVE'
          AND code IS NOT NULL
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_one(pool)
    .await
    .context("failed to compute active DTC count for native postgres health modifier")?;

    let dtc_count: i64 = dtc_row
        .try_get("dtc_count")
        .context("failed to parse native postgres active DTC count")?;

    let mil_row = sqlx::query(
        r#"
        SELECT event_type
        FROM vehicle_diagnostic_event
        WHERE vehicle_uid = $1
          AND observed_at >= $2
          AND event_type IN ('MIL_ON', 'MIL_OFF')
        ORDER BY observed_at DESC
        LIMIT 1
        "#,
    )
    .bind(vehicle_uid)
    .bind(cutoff.to_rfc3339())
    .fetch_optional(pool)
    .await
    .context("failed to load native postgres MIL status for health modifier")?;

    let mil_event_type = mil_row.and_then(|row| row.try_get::<String, _>("event_type").ok());
    let mil_on = mil_event_type
        .as_deref()
        .map(|event_type| event_type == "MIL_ON")
        .unwrap_or(false);

    let mil_penalty = if mil_on { 6.0 } else { 0.0 };
    let dtc_penalty = (dtc_count.max(0) as f64 * 0.5).min(4.0);
    let penalty = (mil_penalty + dtc_penalty).min(10.0);

    let sample_count = dtc_count.max(0) + if mil_event_type.is_some() { 1 } else { 0 };
    Ok((penalty, sample_count.max(1)))
}
