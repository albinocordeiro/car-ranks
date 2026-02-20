use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

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

        let ended_at = ended_at_opt.clone().unwrap_or_else(crate::now_str);

        let obs_rows = sqlx::query(
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
        .bind(&ended_at)
        .fetch_all(pool)
        .await
        .context("failed to fetch observations for charging session")?;

        let mut soc_series: Vec<(String, f64)> = Vec::new();
        let mut power_series: Vec<f64> = Vec::new();
        let mut ambient_temps = Vec::new();
        let mut battery_temps = Vec::new();
        let mut charger_type = "unknown".to_string();

        for obs in obs_rows {
            let signal_key: String = obs.try_get("signal_key")?;
            let observed_at: String = obs.try_get("observed_at")?;
            let value_number: Option<f64> = obs.try_get("value_number")?;
            let value_string: Option<String> = obs.try_get("value_string")?;

            match signal_key.as_str() {
                "ev.soc_pct" => {
                    if let Some(v) = value_number {
                        soc_series.push((observed_at, v));
                    }
                }
                "ev.charge_power_kw" | "power.battery_power_kw" => {
                    if let Some(v) = value_number {
                        if v.is_finite() {
                            power_series.push(v.abs());
                        }
                    }
                }
                "environment.ambient_temp_c" => {
                    if let Some(v) = value_number {
                        ambient_temps.push(v);
                    }
                }
                "ev.battery_temp_c" => {
                    if let Some(v) = value_number {
                        battery_temps.push(v);
                    }
                }
                "ev.charger_type" => {
                    if let Some(v) = value_string {
                        charger_type = crate::normalize_charger_type(&v).to_string();
                    }
                }
                _ => {}
            }
        }

        soc_series.sort_by(|a, b| a.0.cmp(&b.0));
        let soc_start = soc_series.first().map(|(_, v)| *v);
        let soc_end = soc_series.last().map(|(_, v)| *v);
        let soc_delta = match (soc_start, soc_end) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        };

        let avg_power = crate::metrics::mean(&power_series);
        let peak_power = crate::metrics::max_value(&power_series);
        let ambient_avg = crate::metrics::mean(&ambient_temps);
        let battery_avg = crate::metrics::mean(&battery_temps);

        let temperature_source = ambient_avg.or(battery_avg);
        let temperature_bin = temperature_source
            .map(crate::derive_temperature_bin)
            .map(str::to_string);

        let duration_hours = match (
            crate::parse_ts(&started_at),
            ended_at_opt.as_deref().and_then(crate::parse_ts),
        ) {
            (Some(start), Some(end)) if end > start => (end - start).num_seconds() as f64 / 3600.0,
            _ => 0.0,
        };

        let energy_added_kwh = avg_power.map(|p| p * duration_hours.max(0.0));
        let status = if ended_at_opt.is_some() {
            "complete"
        } else {
            "partial"
        };

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
        .bind(&vehicle_uid)
        .bind(&session_id)
        .bind(&started_at)
        .bind(ended_at_opt)
        .bind(status)
        .bind(charger_type)
        .bind(soc_start)
        .bind(soc_end)
        .bind(soc_delta)
        .bind(energy_added_kwh)
        .bind(avg_power)
        .bind(peak_power)
        .bind(ambient_avg)
        .bind(battery_avg)
        .bind(temperature_bin)
        .bind(0_i64)
        .bind(power_series.len() as i64)
        .bind(crate::now_str())
        .bind(crate::now_str())
        .execute(pool)
        .await
        .context("failed to upsert charging session")?;

        upserted += 1;
    }

    Ok(upserted)
}
