use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{ApiError, JobResponse, MetricCalc};

#[derive(Debug)]
struct VehicleRankingSeed {
    vehicle_uid: String,
    make: String,
    model: String,
    trim: String,
    model_year: Option<i64>,
    range_retention: Option<f64>,
    sensitivity: Option<f64>,
    charge_retention: Option<f64>,
    confidence_level: String,
}

pub(crate) async fn run_kpi_job(pool: &SqlitePool) -> Result<JobResponse, ApiError> {
    let job_id = Uuid::new_v4().to_string();

    // Rebuild charging sessions first so downstream KPI jobs read the latest aggregates.
    let charging_sessions_upserted = build_charging_sessions(pool)
        .await
        .context("failed to build charging sessions")?;

    let (kpi_rows_upserted, recomputed_vehicles) = recompute_all_kpis(pool)
        .await
        .context("failed to recompute KPIs")?;

    let ranking_rows_upserted = rebuild_all_rankings(pool)
        .await
        .context("failed to rebuild ranking snapshots for all ranking types")?;

    Ok(JobResponse {
        ok: true,
        job_id,
        charging_sessions_upserted,
        kpi_rows_upserted,
        ranking_rows_upserted,
        recomputed_vehicles,
    })
}

pub(crate) async fn recompute_all_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let (temp_rows, temp_vehicles) = recompute_temperature_kpis(pool).await?;
    let (other_rows, other_vehicles) = recompute_non_temperature_kpis(pool).await?;
    Ok((temp_rows + other_rows, temp_vehicles.max(other_vehicles)))
}

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

pub(crate) async fn recompute_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles")?;

    let mut rows_inserted = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        sqlx::query(
            r#"
            DELETE FROM vehicle_kpi_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear KPI snapshots for timeframe {}", timeframe))?;
    }

    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in vehicles list")?;

        for timeframe in ["30d", "90d", "180d"] {
            let cutoff = crate::timeframe_cutoff(timeframe)?;
            let metrics =
                crate::metrics::compute_vehicle_metrics(pool, &vehicle_uid, cutoff).await?;
            let snapshot_ts = crate::now_str();

            for metric in metrics {
                for temp_bin in ["all", "cold"] {
                    insert_kpi_snapshot(
                        pool,
                        &vehicle_uid,
                        "ev_temperature_impact",
                        timeframe,
                        &metric,
                        temp_bin,
                        Some("mild"),
                        Some("cold"),
                        &snapshot_ts,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "failed to insert temperature KPI {} for vehicle {} timeframe {}",
                            metric.key, vehicle_uid, timeframe
                        )
                    })?;

                    rows_inserted += 1;
                }
            }
        }
    }

    Ok((rows_inserted, vehicles.len()))
}

pub(crate) async fn recompute_non_temperature_kpis(pool: &SqlitePool) -> Result<(usize, usize)> {
    let vehicles = sqlx::query(
        r#"
        SELECT vehicle_uid
        FROM vehicle
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to read vehicles for non-temperature KPIs")?;

    let mut rows_inserted = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
        ] {
            sqlx::query(
                r#"
                DELETE FROM vehicle_kpi_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear KPI snapshots for ranking_type {} timeframe {}",
                    ranking_type, timeframe
                )
            })?;
        }
    }

    for vehicle_row in &vehicles {
        let vehicle_uid: String = vehicle_row
            .try_get("vehicle_uid")
            .context("invalid vehicle_uid in non-temperature KPI pass")?;

        for timeframe in ["30d", "90d", "180d"] {
            let cutoff = crate::timeframe_cutoff(timeframe)?;
            let range_metrics =
                crate::metrics::compute_range_efficiency_metrics(pool, &vehicle_uid, cutoff)
                    .await?;
            let charging_metrics =
                crate::metrics::compute_charging_performance_metrics(pool, &vehicle_uid, cutoff)
                    .await?;
            let composite_metrics = crate::metrics::compute_composite_metrics(
                pool,
                &vehicle_uid,
                cutoff,
                &range_metrics,
                &charging_metrics,
            )
            .await?;

            let snapshot_ts = crate::now_str();
            for metric in &range_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_range_efficiency",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }

            for metric in &charging_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_charging_performance",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }

            for metric in &composite_metrics {
                insert_kpi_snapshot(
                    pool,
                    &vehicle_uid,
                    "ev_composite",
                    timeframe,
                    metric,
                    "all",
                    None,
                    None,
                    &snapshot_ts,
                )
                .await?;
                rows_inserted += 1;
            }
        }
    }

    Ok((rows_inserted, vehicles.len()))
}

pub(crate) async fn insert_kpi_snapshot(
    pool: &SqlitePool,
    vehicle_uid: &str,
    ranking_type: &str,
    timeframe: &str,
    metric: &MetricCalc,
    temperature_bin: &str,
    baseline_temperature_bin: Option<&str>,
    compare_temperature_bin: Option<&str>,
    snapshot_ts: &str,
) -> Result<()> {
    let Some((formula, required_signals, optional_signals)) =
        crate::kpi_specs::locked_kpi_spec_details(ranking_type, metric.key)
    else {
        return Err(anyhow::anyhow!(
            "kpi_key {} is not locked for ranking_type {}",
            metric.key,
            ranking_type
        ));
    };
    if metric.sample_count < 0 {
        return Err(anyhow::anyhow!(
            "kpi_key {} has invalid negative sample_count {}",
            metric.key,
            metric.sample_count
        ));
    }

    tracing::debug!(
        ranking_type,
        kpi_key = metric.key,
        formula,
        ?required_signals,
        ?optional_signals,
        "persisting locked KPI snapshot"
    );

    sqlx::query(
        r#"
        INSERT INTO vehicle_kpi_snapshot (
            snapshot_id,
            vehicle_uid,
            ranking_type,
            timeframe,
            kpi_key,
            kpi_value,
            kpi_unit,
            direction,
            confidence_level,
            sample_count,
            temperature_bin,
            baseline_temperature_bin,
            compare_temperature_bin,
            computed_at,
            source_job_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(vehicle_uid)
    .bind(ranking_type)
    .bind(timeframe)
    .bind(metric.key)
    .bind(metric.value)
    .bind(metric.unit)
    .bind(metric.direction)
    .bind(metric.confidence_level)
    .bind(metric.sample_count)
    .bind(temperature_bin)
    .bind(baseline_temperature_bin)
    .bind(compare_temperature_bin)
    .bind(snapshot_ts)
    .bind("internal_recompute")
    .execute(pool)
    .await
    .context("failed to insert KPI snapshot row")?;
    Ok(())
}

pub(crate) async fn rebuild_all_rankings(pool: &SqlitePool) -> Result<usize> {
    let temp_rows = rebuild_temperature_rankings(pool).await?;
    let non_temp_rows = rebuild_non_temperature_rankings(pool).await?;
    Ok(temp_rows + non_temp_rows)
}

pub(crate) async fn rebuild_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    for timeframe in ["30d", "90d", "180d"] {
        let ranking_snapshot_ts = crate::now_str();
        sqlx::query(
            r#"
            DELETE FROM cohort_ranking_snapshot
            WHERE ranking_type = 'ev_temperature_impact'
              AND timeframe = ?
            "#,
        )
        .bind(timeframe)
        .execute(pool)
        .await
        .with_context(|| format!("failed to clear rankings for timeframe {}", timeframe))?;

        let rows = sqlx::query(
            r#"
            SELECT
              v.vehicle_uid,
              COALESCE(v.make, 'unknown') AS make,
              COALESCE(v.model, 'unknown') AS model,
              COALESCE(v.trim, 'unknown') AS trim,
              v.model_year,
              MAX(CASE WHEN k.kpi_key = 'cold_weather_range_retention' THEN k.kpi_value END) AS range_retention,
              MAX(CASE WHEN k.kpi_key = 'range_temperature_sensitivity_index' THEN k.kpi_value END) AS sensitivity,
              MAX(CASE WHEN k.kpi_key = 'cold_weather_charge_speed_retention' THEN k.kpi_value END) AS charge_retention
            FROM vehicle v
            LEFT JOIN vehicle_kpi_snapshot k
              ON k.vehicle_uid = v.vehicle_uid
             AND k.ranking_type = 'ev_temperature_impact'
             AND k.timeframe = ?
             AND k.temperature_bin = 'cold'
            GROUP BY v.vehicle_uid, v.make, v.model, v.trim, v.model_year
            "#,
        )
        .bind(timeframe)
        .fetch_all(pool)
        .await
        .with_context(|| format!("failed to fetch KPI seeds for timeframe {}", timeframe))?;

        let mut seeds = Vec::new();
        for row in rows {
            let vehicle_uid: String = row.try_get("vehicle_uid")?;
            let make: String = row.try_get("make")?;
            let model: String = row.try_get("model")?;
            let trim: String = row.try_get("trim")?;
            let model_year: Option<i64> = row.try_get("model_year")?;
            let range_retention: Option<f64> = row.try_get("range_retention")?;
            let sensitivity: Option<f64> = row.try_get("sensitivity")?;
            let charge_retention: Option<f64> = row.try_get("charge_retention")?;

            // Temperature impact rankings require both gated retention metrics.
            if range_retention.is_none() || charge_retention.is_none() {
                continue;
            }

            let confidence_level = if sensitivity.is_some() {
                "stable"
            } else {
                "medium"
            }
            .to_string();

            seeds.push(VehicleRankingSeed {
                vehicle_uid,
                make,
                model,
                trim,
                model_year,
                range_retention,
                sensitivity,
                charge_retention,
                confidence_level,
            });
        }

        let mut cohorts: HashMap<String, Vec<(VehicleRankingSeed, f64)>> = HashMap::new();

        for seed in seeds {
            let score = crate::metrics::score_temperature_impact(
                seed.range_retention,
                seed.charge_retention,
                seed.sensitivity,
            );
            let cohort_key = format!(
                "bev|{}|{}|{}|{}",
                seed.make,
                seed.model,
                seed.trim,
                crate::year_band(seed.model_year)
            );
            cohorts.entry(cohort_key).or_default().push((seed, score));
        }

        for (cohort_key, entries) in cohorts {
            let mut entries = entries;
            entries.sort_by(|a, b| crate::cmp_f64_desc(a.1, b.1));
            let cohort_size = entries.len() as i64;
            let sample_gate_passed = cohort_size >= 10;

            for (index, (seed, score)) in entries.into_iter().enumerate() {
                for bin in ["all", "cold"] {
                    sqlx::query(
                        r#"
                        INSERT INTO cohort_ranking_snapshot (
                            ranking_snapshot_id,
                            ranking_type,
                            timeframe,
                            temperature_bin,
                            cohort_key,
                            cohort_size,
                            sample_gate_passed,
                            vehicle_uid,
                            rank_position,
                            score,
                            confidence_level,
                            computed_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind("ev_temperature_impact")
                    .bind(timeframe)
                    .bind(bin)
                    .bind(&cohort_key)
                    .bind(cohort_size)
                    .bind(i64::from(sample_gate_passed))
                    .bind(&seed.vehicle_uid)
                    .bind((index + 1) as i64)
                    .bind(score)
                    .bind(&seed.confidence_level)
                    .bind(&ranking_snapshot_ts)
                    .execute(pool)
                    .await
                    .context("failed to insert cohort ranking snapshot")?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}

pub(crate) async fn rebuild_non_temperature_rankings(pool: &SqlitePool) -> Result<usize> {
    let mut upserted_rows = 0usize;

    let vehicle_rows = sqlx::query(
        r#"
        SELECT
          vehicle_uid,
          COALESCE(make, 'unknown') AS make,
          COALESCE(model, 'unknown') AS model,
          COALESCE(trim, 'unknown') AS trim,
          model_year
        FROM vehicle
        ORDER BY vehicle_uid
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to fetch vehicles for non-temperature rankings")?;

    for timeframe in ["30d", "90d", "180d"] {
        for ranking_type in [
            "ev_range_efficiency",
            "ev_charging_performance",
            "ev_composite",
        ] {
            sqlx::query(
                r#"
                DELETE FROM cohort_ranking_snapshot
                WHERE ranking_type = ?
                  AND timeframe = ?
                "#,
            )
            .bind(ranking_type)
            .bind(timeframe)
            .execute(pool)
            .await
            .with_context(|| {
                format!(
                    "failed to clear ranking snapshots for {} {}",
                    ranking_type, timeframe
                )
            })?;

            let ranking_snapshot_ts = crate::now_str();
            let mut cohorts: HashMap<String, Vec<(String, f64, String, BTreeMap<String, f64>)>> =
                HashMap::new();

            for row in &vehicle_rows {
                let vehicle_uid: String = row.try_get("vehicle_uid")?;
                let make: String = row.try_get("make")?;
                let model: String = row.try_get("model")?;
                let trim: String = row.try_get("trim")?;
                let model_year: Option<i64> = row.try_get("model_year")?;

                let kpis = crate::kpis::fetch_latest_vehicle_kpis_sqlite(
                    pool,
                    &vehicle_uid,
                    ranking_type,
                    timeframe,
                    "all",
                )
                .await?;
                if kpis.is_empty() {
                    continue;
                }

                let kpi_map: BTreeMap<String, f64> =
                    kpis.iter().map(|k| (k.kpi_key.clone(), k.value)).collect();

                let score = crate::metrics::score_from_kpi_map(ranking_type, &kpi_map);
                let confidence_level =
                    crate::metrics::confidence_from_kpi_metrics(&kpis).to_string();
                let cohort_key = format!(
                    "bev|{}|{}|{}|{}",
                    make,
                    model,
                    trim,
                    crate::year_band(model_year)
                );

                cohorts.entry(cohort_key).or_default().push((
                    vehicle_uid,
                    score,
                    confidence_level,
                    kpi_map,
                ));
            }

            for (cohort_key, mut entries) in cohorts {
                entries.sort_by(|a, b| crate::cmp_f64_desc(a.1, b.1));
                let cohort_size = entries.len() as i64;
                let sample_gate_passed = cohort_size >= 10;

                for (index, (vehicle_uid, score, confidence_level, _kpis)) in
                    entries.into_iter().enumerate()
                {
                    sqlx::query(
                        r#"
                        INSERT INTO cohort_ranking_snapshot (
                            ranking_snapshot_id,
                            ranking_type,
                            timeframe,
                            temperature_bin,
                            cohort_key,
                            cohort_size,
                            sample_gate_passed,
                            vehicle_uid,
                            rank_position,
                            score,
                            confidence_level,
                            computed_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(ranking_type)
                    .bind(timeframe)
                    .bind("all")
                    .bind(&cohort_key)
                    .bind(cohort_size)
                    .bind(i64::from(sample_gate_passed))
                    .bind(vehicle_uid)
                    .bind((index + 1) as i64)
                    .bind(score)
                    .bind(confidence_level)
                    .bind(&ranking_snapshot_ts)
                    .execute(pool)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to insert non-temperature ranking row for {} {}",
                            ranking_type, timeframe
                        )
                    })?;

                    upserted_rows += 1;
                }
            }
        }
    }

    Ok(upserted_rows)
}
