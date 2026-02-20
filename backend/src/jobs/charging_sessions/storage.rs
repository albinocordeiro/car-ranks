use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::session_metrics::SessionMetrics;

/// Fully-validated payload for one `vehicle_charging_session` upsert.
pub(super) struct SessionUpsert<'a> {
    pub(super) vehicle_uid: &'a str,
    pub(super) session_id: &'a str,
    pub(super) started_at: &'a str,
    pub(super) ended_at: Option<&'a str>,
    pub(super) status: &'a str,
    pub(super) metrics: &'a SessionMetrics,
}

/// Persists one charging-session aggregate row.
pub(super) async fn upsert_charging_session(
    pool: &SqlitePool,
    payload: SessionUpsert<'_>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO vehicle_charging_session (
            charging_session_id,
            vehicle_uid,
            session_id,
            started_at,
            ended_at,
            status,
            charger_type,
            soc_start_pct,
            soc_end_pct,
            soc_delta_pct,
            energy_added_kwh,
            avg_charge_power_kw,
            peak_charge_power_kw,
            ambient_temp_avg_c,
            battery_temp_avg_c,
            temperature_bin,
            temperature_is_estimated,
            sample_count,
            created_at,
            updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_id) DO UPDATE SET
            started_at = excluded.started_at,
            ended_at = excluded.ended_at,
            status = excluded.status,
            charger_type = excluded.charger_type,
            soc_start_pct = excluded.soc_start_pct,
            soc_end_pct = excluded.soc_end_pct,
            soc_delta_pct = excluded.soc_delta_pct,
            energy_added_kwh = excluded.energy_added_kwh,
            avg_charge_power_kw = excluded.avg_charge_power_kw,
            peak_charge_power_kw = excluded.peak_charge_power_kw,
            ambient_temp_avg_c = excluded.ambient_temp_avg_c,
            battery_temp_avg_c = excluded.battery_temp_avg_c,
            temperature_bin = excluded.temperature_bin,
            temperature_is_estimated = excluded.temperature_is_estimated,
            sample_count = excluded.sample_count,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(payload.vehicle_uid)
    .bind(payload.session_id)
    .bind(payload.started_at)
    .bind(payload.ended_at)
    .bind(payload.status)
    .bind(&payload.metrics.charger_type)
    .bind(payload.metrics.soc_start)
    .bind(payload.metrics.soc_end)
    .bind(payload.metrics.soc_delta)
    .bind(payload.metrics.energy_added_kwh)
    .bind(payload.metrics.avg_power)
    .bind(payload.metrics.peak_power)
    .bind(payload.metrics.ambient_avg)
    .bind(payload.metrics.battery_avg)
    .bind(&payload.metrics.temperature_bin)
    .bind(0_i64)
    .bind(payload.metrics.sample_count)
    .bind(crate::now_str())
    .bind(crate::now_str())
    .execute(pool)
    .await?;

    Ok(())
}
