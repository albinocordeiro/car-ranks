use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

use super::*;

#[tokio::test]
async fn temperature_rankings_skip_vehicle_when_range_gate_fails() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;
    crate::migrations::apply_schema(&pool).await?;

    let vehicle_uid = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO vehicle (
          vehicle_uid,
          source_account_id,
          powertrain_class,
          created_at,
          updated_at
        ) VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&vehicle_uid)
    .bind("test-account")
    .bind("bev")
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await
    .context("failed to insert vehicle")?;

    for i in 0..6 {
        let ts = now - Duration::hours(4) + Duration::minutes(i * 5);
        let odo_km = 1000.0 + i as f64;
        let soc_pct = 90.0 - (i as f64 * 0.5);
        let temp_c = if i < 3 { 20.0 } else { 0.0 };

        for (signal_key, value, temp_bin) in [
            ("distance.odometer", odo_km, None),
            ("ev.soc_pct", soc_pct, None),
            (
                "environment.ambient_temp_c",
                temp_c,
                Some(derive_temperature_bin(temp_c).to_string()),
            ),
        ] {
            sqlx::query(
                r#"
                INSERT INTO vehicle_signal_observation (
                  observation_id,
                  vehicle_uid,
                  signal_key,
                  value_number,
                  observed_at,
                  ingested_at,
                  source,
                  status,
                  temperature_bin,
                  is_temperature_estimated
                ) VALUES (?, ?, ?, ?, ?, ?, 'OBD', 'ok', ?, 0)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&vehicle_uid)
            .bind(signal_key)
            .bind(value)
            .bind(ts.to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(temp_bin)
            .execute(&pool)
            .await
            .with_context(|| format!("failed to insert observation {}", signal_key))?;
        }
    }

    for (session_id, avg_power_kw, temp_bin) in [
        (Uuid::new_v4().to_string(), 62.0, "mild"),
        (Uuid::new_v4().to_string(), 34.0, "cold"),
    ] {
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
              avg_charge_power_kw,
              temperature_bin,
              temperature_is_estimated,
              sample_count,
              created_at,
              updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid)
        .bind(session_id)
        .bind((now - Duration::hours(2)).to_rfc3339())
        .bind((now - Duration::hours(1)).to_rfc3339())
        .bind("complete")
        .bind("dc")
        .bind(avg_power_kw)
        .bind(temp_bin)
        .bind(0_i64)
        .bind(2_i64)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await
        .context("failed to insert charging session")?;
    }

    let _ = crate::jobs::recompute_temperature_kpis(&pool).await?;
    let _ = crate::jobs::rebuild_temperature_rankings(&pool).await?;

    let temp_keys: HashSet<String> = sqlx::query(
        r#"
        SELECT DISTINCT kpi_key
        FROM vehicle_kpi_snapshot
        WHERE vehicle_uid = ?
          AND ranking_type = 'ev_temperature_impact'
          AND timeframe = '30d'
          AND temperature_bin = 'cold'
        "#,
    )
    .bind(&vehicle_uid)
    .fetch_all(&pool)
    .await
    .context("failed to fetch temperature KPI keys")?
    .into_iter()
    .map(|row| row.try_get::<String, _>("kpi_key"))
    .collect::<std::result::Result<HashSet<_>, _>>()
    .context("failed to parse temperature KPI keys")?;

    assert!(!temp_keys.contains("cold_weather_range_retention"));
    assert!(!temp_keys.contains("range_temperature_sensitivity_index"));
    assert!(temp_keys.contains("cold_weather_charge_speed_retention"));

    let ranking_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM cohort_ranking_snapshot
        WHERE ranking_type = 'ev_temperature_impact'
          AND timeframe = '30d'
        "#,
    )
    .fetch_one(&pool)
    .await
    .context("failed to count temperature rankings")?;

    assert_eq!(ranking_count, 0);
    Ok(())
}

#[tokio::test]
async fn end_to_end_kpi_job_materializes_locked_kpi_sets() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;
    crate::migrations::apply_schema(&pool).await?;

    let state = AppState {
        sqlite_pool: pool.clone(),
        pg_pool: None,
        backend: DatabaseBackend::Sqlite,
        signal_keys: Arc::new(load_signal_keys()?),
    };

    let vehicle_uid = Uuid::new_v4();
    let mild_charge_session_id = Uuid::new_v4();
    let cold_charge_session_id = Uuid::new_v4();
    let now = Utc::now();

    let drive_start = now - Duration::hours(6);
    let mild_charge_start = now - Duration::hours(3);
    let mild_charge_stop = now - Duration::hours(2) - Duration::minutes(30);
    let cold_charge_start = now - Duration::hours(2);
    let cold_charge_stop = now - Duration::hours(1) - Duration::minutes(20);

    let number_record = |observed_at: DateTime<Utc>,
                         signal_key: &str,
                         value_number: f64,
                         unit: Option<&str>,
                         session_id: Option<Uuid>|
     -> TelemetryRecord {
        TelemetryRecord {
            observed_at,
            signal_key: signal_key.to_string(),
            value_number: Some(value_number),
            value_string: None,
            value_bool: None,
            value_json: None,
            unit: unit.map(str::to_string),
            status: "ok".to_string(),
            confidence: Some(1.0),
            source_signal: Some(signal_key.to_string()),
            freshness_ttl_seconds: Some(30),
            temperature_bin: if signal_key == "environment.ambient_temp_c" {
                Some(derive_temperature_bin(value_number).to_string())
            } else {
                None
            },
            is_temperature_estimated: Some(false),
            session_id,
            raw_payload_ref: None,
        }
    };

    let string_record = |observed_at: DateTime<Utc>,
                         signal_key: &str,
                         value_string: &str,
                         session_id: Option<Uuid>|
     -> TelemetryRecord {
        TelemetryRecord {
            observed_at,
            signal_key: signal_key.to_string(),
            value_number: None,
            value_string: Some(value_string.to_string()),
            value_bool: None,
            value_json: None,
            unit: None,
            status: "ok".to_string(),
            confidence: Some(1.0),
            source_signal: Some(signal_key.to_string()),
            freshness_ttl_seconds: Some(60),
            temperature_bin: None,
            is_temperature_estimated: Some(false),
            session_id,
            raw_payload_ref: None,
        }
    };

    let mut records = Vec::new();
    for i in 0..21 {
        let ts = drive_start + Duration::minutes(i * 5);
        let odo_km = 1000.0 + (i as f64 * 2.5);
        let soc_pct = 90.0 - (i as f64 * 0.35);
        let ambient_temp_c = if i < 11 { 20.0 } else { 0.0 };
        let speed_kmh = if i % 2 == 0 { 35.0 } else { 95.0 };
        let regen_kw = 4.0 + (i % 2) as f64;
        let traction_kw = 18.0 + (i % 3) as f64;

        records.push(number_record(
            ts,
            "distance.odometer",
            odo_km,
            Some("km"),
            None,
        ));
        records.push(number_record(ts, "ev.soc_pct", soc_pct, Some("%"), None));
        records.push(number_record(
            ts,
            "environment.ambient_temp_c",
            ambient_temp_c,
            Some("C"),
            None,
        ));
        records.push(number_record(
            ts,
            "speed.vehicle",
            speed_kmh,
            Some("km/h"),
            None,
        ));
        records.push(number_record(
            ts,
            "ev.regen_power_kw",
            regen_kw,
            Some("kW"),
            None,
        ));
        records.push(number_record(
            ts,
            "ev.traction_power_kw",
            traction_kw,
            Some("kW"),
            None,
        ));
    }

    for (session_id, start, stop, power_a, power_b, temp_a, temp_b, soc_a, soc_b) in [
        (
            mild_charge_session_id,
            mild_charge_start,
            mild_charge_stop,
            62.0,
            58.0,
            20.0,
            21.0,
            40.0,
            50.0,
        ),
        (
            cold_charge_session_id,
            cold_charge_start,
            cold_charge_stop,
            36.0,
            34.0,
            0.0,
            1.0,
            52.0,
            60.0,
        ),
    ] {
        let mid = start + Duration::minutes(10);
        let near_end = stop - Duration::minutes(5);

        records.push(number_record(
            start,
            "ev.soc_pct",
            soc_a,
            Some("%"),
            Some(session_id),
        ));
        records.push(number_record(
            near_end,
            "ev.soc_pct",
            soc_b,
            Some("%"),
            Some(session_id),
        ));
        records.push(number_record(
            start,
            "ev.charge_power_kw",
            power_a,
            Some("kW"),
            Some(session_id),
        ));
        records.push(number_record(
            mid,
            "ev.charge_power_kw",
            power_b,
            Some("kW"),
            Some(session_id),
        ));
        records.push(number_record(
            start,
            "environment.ambient_temp_c",
            temp_a,
            Some("C"),
            Some(session_id),
        ));
        records.push(number_record(
            near_end,
            "environment.ambient_temp_c",
            temp_b,
            Some("C"),
            Some(session_id),
        ));
        records.push(number_record(
            mid,
            "ev.battery_temp_c",
            if temp_a > 10.0 { 25.0 } else { 6.0 },
            Some("C"),
            Some(session_id),
        ));
        records.push(string_record(
            mid,
            "ev.charger_type",
            "dc_fast",
            Some(session_id),
        ));
        records.push(string_record(
            start,
            "ev.charging_state",
            "charging",
            Some(session_id),
        ));
    }

    let payload = TelemetryBatchRequest {
        batch_id: Uuid::new_v4(),
        schema_version: "0.2".to_string(),
        vehicle_uid,
        source: "OBD".to_string(),
        client: Some(ClientInfo {
            platform: Some("ios".to_string()),
            app_version: Some("1.0.0-test".to_string()),
            adapter_fingerprint: Some("adapter-test-123".to_string()),
        }),
        capture_window: CaptureWindow {
            started_at: drive_start - Duration::minutes(1),
            ended_at: now,
            sample_interval_seconds: Some(60),
        },
        records,
        session_events: vec![
            SessionEventInput {
                event_type: "charging_session_start".to_string(),
                observed_at: mild_charge_start,
                session_id: mild_charge_session_id,
            },
            SessionEventInput {
                event_type: "charging_session_stop".to_string(),
                observed_at: mild_charge_stop,
                session_id: mild_charge_session_id,
            },
            SessionEventInput {
                event_type: "charging_session_start".to_string(),
                observed_at: cold_charge_start,
                session_id: cold_charge_session_id,
            },
            SessionEventInput {
                event_type: "charging_session_stop".to_string(),
                observed_at: cold_charge_stop,
                session_id: cold_charge_session_id,
            },
        ],
        diagnostics: vec![DiagnosticInput {
            observed_at: now - Duration::minutes(45),
            mil_on: Some(true),
            dtcs_active: Some(vec!["P0ABC".to_string(), "P0DEF".to_string()]),
        }],
    };

    let Json(ingest_response) = crate::handlers::post_telemetry_batches(
        State(state.clone()),
        crate::auth::AuthContext::from_user_id(Uuid::new_v4()),
        Json(payload),
    )
    .await
    .map_err(|err| anyhow::anyhow!("ingest failed: {} {}", err.error, err.message))?;
    assert!(ingest_response.accepted);
    assert!(!ingest_response.duplicate);
    assert_eq!(ingest_response.records_rejected, 0);

    let job = crate::jobs::run_kpi_job(&pool)
        .await
        .map_err(|err| anyhow::anyhow!("kpi job failed: {} {}", err.error, err.message))?;
    assert!(job.ok);
    assert_eq!(job.recomputed_vehicles, 1);
    assert!(job.kpi_rows_upserted > 0);
    assert!(job.ranking_rows_upserted > 0);

    let vehicle_uid_text = vehicle_uid.to_string();
    let expected_by_ranking: [(&str, &[&str]); 4] = [
        (
            "ev_range_efficiency",
            &[
                "ev_net_energy_efficiency",
                "ev_estimated_practical_range",
                "ev_urban_efficiency",
                "ev_highway_efficiency",
                "regeneration_recovery_ratio",
                "soc_depletion_rate_per_100km",
                "ev_range_efficiency_score",
            ],
        ),
        (
            "ev_charging_performance",
            &[
                "temp_adjusted_charge_acceptance_score",
                "cold_weather_charge_speed_retention",
                "charging_performance_score",
            ],
        ),
        (
            "ev_temperature_impact",
            &[
                "cold_weather_range_retention",
                "range_temperature_sensitivity_index",
                "cold_weather_charge_speed_retention",
            ],
        ),
        (
            "ev_composite",
            &[
                "ev_composite_base_score",
                "ev_health_modifier_penalty",
                "ev_composite_score",
            ],
        ),
    ];

    for (ranking_type, expected_keys) in expected_by_ranking {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT kpi_key
            FROM vehicle_kpi_snapshot
            WHERE vehicle_uid = ?
              AND ranking_type = ?
              AND timeframe = '30d'
            "#,
        )
        .bind(&vehicle_uid_text)
        .bind(ranking_type)
        .fetch_all(&pool)
        .await
        .with_context(|| format!("failed to fetch keys for {}", ranking_type))?;

        let keys: HashSet<String> = rows
            .into_iter()
            .map(|row| row.try_get::<String, _>("kpi_key"))
            .collect::<std::result::Result<HashSet<_>, _>>()
            .with_context(|| format!("failed to parse keys for {}", ranking_type))?;

        for expected_key in expected_keys {
            assert!(
                keys.contains(*expected_key),
                "missing {} for ranking_type {}",
                expected_key,
                ranking_type
            );
        }
    }

    let composite_rows = sqlx::query(
        r#"
        SELECT kpi_key, kpi_value
        FROM vehicle_kpi_snapshot
        WHERE vehicle_uid = ?
          AND ranking_type = 'ev_composite'
          AND timeframe = '30d'
        "#,
    )
    .bind(&vehicle_uid_text)
    .fetch_all(&pool)
    .await
    .context("failed to fetch composite KPI values")?;

    let mut composite_map = BTreeMap::new();
    for row in composite_rows {
        let key: String = row.try_get("kpi_key")?;
        let value: f64 = row.try_get("kpi_value")?;
        composite_map.insert(key, value);
    }

    let base = *composite_map
        .get("ev_composite_base_score")
        .context("missing ev_composite_base_score")?;
    let penalty = *composite_map
        .get("ev_health_modifier_penalty")
        .context("missing ev_health_modifier_penalty")?;
    let final_score = *composite_map
        .get("ev_composite_score")
        .context("missing ev_composite_score")?;

    assert!(penalty > 0.0);
    assert!(final_score <= base);
    assert!((final_score - (base - penalty).clamp(0.0, 100.0)).abs() < 0.0001);

    for ranking_type in [
        "ev_range_efficiency",
        "ev_charging_performance",
        "ev_composite",
        "ev_temperature_impact",
    ] {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM cohort_ranking_snapshot
            WHERE ranking_type = ?
              AND timeframe = '30d'
            "#,
        )
        .bind(ranking_type)
        .fetch_one(&pool)
        .await
        .with_context(|| format!("failed to count rankings for {}", ranking_type))?;

        assert!(count > 0, "expected ranking rows for {}", ranking_type);
    }

    Ok(())
}
