use super::*;
use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use sqlx::Connection;
use sqlx::Executor;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use uuid::Uuid;

#[test]
fn temperature_bin_boundaries() {
    assert_eq!(derive_temperature_bin(-10.0), "very_cold");
    assert_eq!(derive_temperature_bin(-5.0), "very_cold");
    assert_eq!(derive_temperature_bin(-4.9), "cold");
    assert_eq!(derive_temperature_bin(5.0), "cold");
    assert_eq!(derive_temperature_bin(10.0), "cool");
    assert_eq!(derive_temperature_bin(20.0), "mild");
    assert_eq!(derive_temperature_bin(30.0), "hot");
}

#[test]
fn percentile_higher_is_better() {
    let values = vec![50.0, 60.0, 70.0, 80.0];
    assert_eq!(percentile_rank(&values, 70.0, true), 75);
    assert_eq!(percentile_rank(&values, 50.0, true), 25);
}

#[test]
fn percentile_lower_is_better() {
    let values = vec![10.0, 20.0, 30.0, 40.0];
    assert_eq!(percentile_rank(&values, 20.0, false), 75);
    assert_eq!(percentile_rank(&values, 40.0, false), 25);
}

#[test]
fn locked_kpi_catalog_contains_core_composite_metric() {
    let spec = kpi_specs::lookup_kpi_spec("ev_composite", "ev_composite_score");
    assert!(spec.is_some());
}

#[test]
fn wh_per_km_from_soc_delta_works() {
    let wh_per_km = metrics::wh_per_km_from_soc_delta(5.0, 20.0, 60.0).expect("expected value");
    assert!((wh_per_km - 150.0).abs() < 0.0001);
}

#[test]
fn score_from_kpi_map_range_fallback_uses_net_efficiency() {
    let mut kpis = BTreeMap::new();
    kpis.insert("ev_estimated_practical_range".to_string(), 280.0);
    kpis.insert("ev_net_energy_efficiency".to_string(), 160.0);

    let score = metrics::score_from_kpi_map("ev_range_efficiency", &kpis);
    assert!(score > 0.0);
    assert!(score <= 100.0);
}

#[test]
fn sqlite_migration_matches_legacy_schema_snapshot() {
    assert_eq!(SQLITE_MIGRATION_0001, LEGACY_SQLITE_SCHEMA);
}

#[test]
fn postgres_migration_has_expected_base_tables() {
    assert!(!POSTGRES_MIGRATION_0001.contains("PRAGMA"));
    for table_name in [
        "vehicle",
        "ingest_batch",
        "vehicle_signal_observation",
        "vehicle_diagnostic_event",
        "vehicle_session_event",
        "vehicle_charging_session",
        "vehicle_kpi_snapshot",
        "cohort_ranking_snapshot",
    ] {
        let marker = format!("CREATE TABLE IF NOT EXISTS {}", table_name);
        assert!(
            POSTGRES_MIGRATION_0001.contains(&marker),
            "missing table in postgres migration: {}",
            table_name
        );
    }
}

#[tokio::test]
async fn apply_schema_records_migrations_once() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;

    apply_schema(&pool).await?;
    apply_schema(&pool).await?;

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM schema_migration
        WHERE migration_id = '0001_init'
          AND backend = 'sqlite'
        "#,
    )
    .fetch_one(&pool)
    .await
    .context("failed to count applied migrations")?;

    assert_eq!(count, 1);
    Ok(())
}

#[tokio::test]
async fn postgres_bootstrap_migration_applies_when_env_set() -> Result<()> {
    let database_url = match std::env::var("POSTGRES_TEST_DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return Ok(()),
    };

    let schema_name = format!("car_ranks_test_{}", Uuid::new_v4().simple());
    let mut conn = sqlx::postgres::PgConnection::connect(&database_url)
        .await
        .context("failed to connect postgres test database")?;

    conn.execute(format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name).as_str())
        .await
        .context("failed to create postgres test schema")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .context("failed to create postgres test pool")?;
    sqlx::query(format!("SET search_path TO {}", schema_name).as_str())
        .execute(&pool)
        .await
        .context("failed to set postgres search_path")?;

    apply_postgres_schema(&pool).await?;

    let table_exists: Option<String> = sqlx::query_scalar(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = current_schema()
          AND table_name = 'vehicle_kpi_snapshot'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .context("failed to validate postgres migrated tables")?;
    assert_eq!(table_exists.as_deref(), Some("vehicle_kpi_snapshot"));

    sqlx::query("SET search_path TO public")
        .execute(&pool)
        .await
        .context("failed to reset pool search_path")?;
    pool.close().await;

    conn.execute("SET search_path TO public")
        .await
        .context("failed to reset search_path")?;
    conn.execute(format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name).as_str())
        .await
        .context("failed to drop postgres test schema")?;

    Ok(())
}

#[tokio::test]
async fn postgres_kpi_fetch_and_charging_handler_work_when_env_set() -> Result<()> {
    let database_url = match std::env::var("POSTGRES_TEST_DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return Ok(()),
    };

    let schema_name = format!("car_ranks_test_{}", Uuid::new_v4().simple());
    let mut admin_conn = sqlx::postgres::PgConnection::connect(&database_url)
        .await
        .context("failed to connect postgres test database")?;
    admin_conn
        .execute(format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name).as_str())
        .await
        .context("failed to create postgres test schema")?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .context("failed to create postgres test pool")?;
    sqlx::query(format!("SET search_path TO {}", schema_name).as_str())
        .execute(&pool)
        .await
        .context("failed to set postgres search_path")?;
    apply_postgres_schema(&pool).await?;

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
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&vehicle_uid)
    .bind("postgres-test-account")
    .bind("bev")
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await
    .context("failed to insert postgres test vehicle")?;

    let older_ts = (now - Duration::minutes(5)).to_rfc3339();
    let newer_ts = now.to_rfc3339();
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
          computed_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&vehicle_uid)
    .bind("ev_range_efficiency")
    .bind("30d")
    .bind("ev_net_energy_efficiency")
    .bind(190.0_f64)
    .bind("wh_per_km")
    .bind("lower_is_better")
    .bind("medium")
    .bind(12_i64)
    .bind("all")
    .bind(&older_ts)
    .execute(&pool)
    .await
    .context("failed to insert older postgres range KPI")?;
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
          computed_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&vehicle_uid)
    .bind("ev_range_efficiency")
    .bind("30d")
    .bind("ev_net_energy_efficiency")
    .bind(170.0_f64)
    .bind("wh_per_km")
    .bind("lower_is_better")
    .bind("stable")
    .bind(18_i64)
    .bind("all")
    .bind(&newer_ts)
    .execute(&pool)
    .await
    .context("failed to insert newer postgres range KPI")?;
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
          computed_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&vehicle_uid)
    .bind("ev_charging_performance")
    .bind("30d")
    .bind("charging_performance_score")
    .bind(82.0_f64)
    .bind("score")
    .bind("higher_is_better")
    .bind("stable")
    .bind(11_i64)
    .bind("all")
    .bind(&newer_ts)
    .execute(&pool)
    .await
    .context("failed to insert postgres charging KPI")?;

    let fetched = fetch_latest_vehicle_kpis_postgres(
        &pool,
        &vehicle_uid,
        "ev_range_efficiency",
        "30d",
        "all",
    )
    .await
    .context("failed to fetch postgres KPIs")?;
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].kpi_key, "ev_net_energy_efficiency");
    assert!((fetched[0].value - 170.0).abs() < f64::EPSILON);
    assert_eq!(fetched[0].sample_count, 18);

    let sqlite_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to create sqlite state pool")?;
    apply_schema(&sqlite_pool).await?;
    let state = AppState {
        sqlite_pool,
        pg_pool: Some(pool.clone()),
        backend: DatabaseBackend::Postgres,
        signal_keys: Arc::new(load_signal_keys()?),
    };
    let query = KpiQuery {
        vehicle_uid: Uuid::parse_str(&vehicle_uid).context("invalid test vehicle uuid")?,
        timeframe: Some("30d".to_string()),
        temperature_bin: Some("all".to_string()),
        charger_type: Some("all".to_string()),
    };
    let Json(response) = get_kpis_charging(State(state), Query(query))
        .await
        .map_err(|err| anyhow::anyhow!("postgres charging KPI handler failed: {}", err.message))?;
    assert_eq!(response.ranking_type, "ev_charging_performance");
    assert_eq!(response.kpis.len(), 1);
    assert_eq!(response.kpis[0].kpi_key, "charging_performance_score");
    assert!((response.kpis[0].value - 82.0).abs() < f64::EPSILON);

    sqlx::query("SET search_path TO public")
        .execute(&pool)
        .await
        .context("failed to reset pool search_path")?;
    pool.close().await;

    admin_conn
        .execute(format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name).as_str())
        .await
        .context("failed to drop postgres test schema")?;

    Ok(())
}

#[test]
fn temperature_sample_gate_checks() {
    let gates = metrics::TemperatureSampleGates {
        min_cold_distance_km: 20.0,
        min_mild_distance_km: 20.0,
        min_cold_charge_sessions: 1,
        min_mild_charge_sessions: 1,
        min_sensitivity_points: 6,
    };

    assert!(gates.range_gate_passed(20.0, 25.0));
    assert!(!gates.range_gate_passed(19.9, 25.0));
    assert!(!gates.range_gate_passed(20.0, 19.9));

    assert!(gates.charge_gate_passed(1, 1));
    assert!(!gates.charge_gate_passed(0, 1));
    assert!(!gates.charge_gate_passed(1, 0));
}

async fn test_app_state() -> Result<AppState> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;
    apply_schema(&pool).await?;
    Ok(AppState {
        sqlite_pool: pool,
        pg_pool: None,
        backend: DatabaseBackend::Sqlite,
        signal_keys: Arc::new(load_signal_keys()?),
    })
}

fn valid_ingest_payload(
    vehicle_uid: Uuid,
    batch_id: Uuid,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
) -> TelemetryBatchRequest {
    TelemetryBatchRequest {
        batch_id,
        schema_version: INGEST_SCHEMA_VERSION.to_string(),
        vehicle_uid,
        source: "OBD".to_string(),
        client: Some(ClientInfo {
            platform: Some("ios".to_string()),
            app_version: Some("1.0.0-test".to_string()),
            adapter_fingerprint: Some("adapter-test".to_string()),
        }),
        capture_window: CaptureWindow {
            started_at,
            ended_at,
            sample_interval_seconds: Some(60),
        },
        records: vec![TelemetryRecord {
            observed_at: started_at + Duration::seconds(5),
            signal_key: "speed.vehicle".to_string(),
            value_number: Some(42.0),
            value_string: None,
            value_bool: None,
            value_json: None,
            unit: Some("km/h".to_string()),
            status: "ok".to_string(),
            confidence: Some(0.95),
            source_signal: Some("01_0D".to_string()),
            freshness_ttl_seconds: Some(30),
            temperature_bin: None,
            is_temperature_estimated: Some(false),
            session_id: None,
            raw_payload_ref: None,
        }],
        session_events: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[tokio::test]
async fn ingest_duplicate_same_envelope_returns_duplicate_true() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );

    let Json(first_response) = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .map_err(|err| anyhow::anyhow!("first ingest failed: {} {}", err.error, err.message))?;
    assert!(first_response.accepted);
    assert!(!first_response.duplicate);

    let duplicate_payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let Json(duplicate_response) =
        post_telemetry_batches(State(state.clone()), Json(duplicate_payload))
            .await
            .map_err(|err| {
                anyhow::anyhow!("duplicate ingest failed: {} {}", err.error, err.message)
            })?;
    assert!(duplicate_response.accepted);
    assert!(duplicate_response.duplicate);
    assert_eq!(duplicate_response.records_accepted, 0);
    Ok(())
}

#[tokio::test]
async fn ingest_duplicate_with_different_envelope_returns_conflict() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    let _ = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .map_err(|err| anyhow::anyhow!("first ingest failed: {} {}", err.error, err.message))?;

    let conflict_payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::seconds(10),
    );
    let err = post_telemetry_batches(State(state.clone()), Json(conflict_payload))
        .await
        .expect_err("expected idempotency conflict");

    assert_eq!(err.status, StatusCode::CONFLICT);
    assert_eq!(err.error, "conflict");
    Ok(())
}

#[tokio::test]
async fn ingest_rejects_unsupported_schema_version() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let mut payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    payload.schema_version = "1.0".to_string();

    let err = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .expect_err("expected schema_version rejection");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.error, "bad_request");
    Ok(())
}

#[tokio::test]
async fn ingest_rejects_record_outside_capture_window() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let mut payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    payload.records[0].observed_at = now;

    let err = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .expect_err("expected out-of-window rejection");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.error, "bad_request");
    Ok(())
}

#[tokio::test]
async fn ingest_rejects_unknown_session_event_type() -> Result<()> {
    let state = test_app_state().await?;
    let now = Utc::now();
    let vehicle_uid = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let mut payload = valid_ingest_payload(
        vehicle_uid,
        batch_id,
        now - Duration::minutes(2),
        now - Duration::minutes(1),
    );
    payload.session_events.push(SessionEventInput {
        event_type: "invalid_session_event".to_string(),
        observed_at: now - Duration::minutes(1) + Duration::seconds(10),
        session_id: Uuid::new_v4(),
    });

    let err = post_telemetry_batches(State(state.clone()), Json(payload))
        .await
        .expect_err("expected session event type rejection");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert_eq!(err.error, "bad_request");
    Ok(())
}

mod kpi_job;
