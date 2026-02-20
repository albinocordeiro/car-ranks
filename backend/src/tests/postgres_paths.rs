use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Connection, Executor};
use uuid::Uuid;

use super::*;

/// Disposable PostgreSQL schema sandbox used by env-gated integration tests.
struct PostgresTestContext {
    schema_name: String,
    admin_conn: sqlx::postgres::PgConnection,
    pool: sqlx::PgPool,
}

impl PostgresTestContext {
    /// Creates a schema-isolated test context when `POSTGRES_TEST_DATABASE_URL` is set.
    async fn maybe_new() -> Result<Option<Self>> {
        let database_url = match std::env::var("POSTGRES_TEST_DATABASE_URL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(None),
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

        crate::migrations::apply_postgres_schema(&pool).await?;

        Ok(Some(Self {
            schema_name,
            admin_conn,
            pool,
        }))
    }

    /// Builds app state that mirrors runtime Postgres mode (with SQLite job sidecar).
    async fn app_state(&self) -> Result<AppState> {
        let sqlite_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .context("failed to create sqlite state pool")?;
        crate::migrations::apply_schema(&sqlite_pool).await?;
        Ok(AppState {
            sqlite_pool,
            pg_pool: Some(self.pool.clone()),
            backend: DatabaseBackend::Postgres,
            signal_keys: Arc::new(load_signal_keys()?),
        })
    }

    /// Resets search path and drops the disposable schema.
    async fn cleanup(mut self) -> Result<()> {
        sqlx::query("SET search_path TO public")
            .execute(&self.pool)
            .await
            .context("failed to reset pool search_path")?;
        self.pool.close().await;

        self.admin_conn
            .execute("SET search_path TO public")
            .await
            .context("failed to reset admin search_path")?;
        self.admin_conn
            .execute(format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema_name).as_str())
            .await
            .context("failed to drop postgres test schema")?;
        Ok(())
    }
}

fn auth_context(user_id: Uuid) -> crate::auth::AuthContext {
    crate::auth::AuthContext::from_user_id(user_id)
}

fn number_record(
    observed_at: DateTime<Utc>,
    signal_key: &str,
    value_number: f64,
    unit: Option<&str>,
    session_id: Option<Uuid>,
) -> TelemetryRecord {
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
}

fn string_record(
    observed_at: DateTime<Utc>,
    signal_key: &str,
    value_string: &str,
    session_id: Option<Uuid>,
) -> TelemetryRecord {
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
        freshness_ttl_seconds: Some(30),
        temperature_bin: None,
        is_temperature_estimated: Some(false),
        session_id,
        raw_payload_ref: None,
    }
}

fn ingest_payload(
    vehicle_uid: Uuid,
    batch_id: Uuid,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    records: Vec<TelemetryRecord>,
    session_events: Vec<SessionEventInput>,
) -> TelemetryBatchRequest {
    TelemetryBatchRequest {
        batch_id,
        schema_version: crate::ingest::INGEST_SCHEMA_VERSION.to_string(),
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
        records,
        session_events,
        diagnostics: Vec::new(),
    }
}

async fn insert_vehicle_owner_access(
    pool: &sqlx::PgPool,
    vehicle_uid: &str,
    user_id: Uuid,
    now: &str,
    make: &str,
    model: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO vehicle (
          vehicle_uid,
          source_account_id,
          make,
          model,
          powertrain_class,
          created_at,
          updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(vehicle_uid)
    .bind("postgres-test-account")
    .bind(make)
    .bind(model)
    .bind("bev")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("failed to insert postgres test vehicle")?;

    sqlx::query(
        r#"
        INSERT INTO app_user (user_id, created_at, updated_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id.to_string())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("failed to insert postgres app user")?;

    sqlx::query(
        r#"
        INSERT INTO user_vehicle_access (
          user_id,
          vehicle_uid,
          access_role,
          created_at,
          updated_at
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id.to_string())
    .bind(vehicle_uid)
    .bind("owner")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("failed to insert postgres vehicle access")?;

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

    crate::migrations::apply_postgres_schema(&pool).await?;

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
    crate::migrations::apply_postgres_schema(&pool).await?;

    let vehicle_uid = Uuid::new_v4().to_string();
    let auth_user_id = Uuid::new_v4();
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
    sqlx::query(
        r#"
        INSERT INTO app_user (
          user_id,
          created_at,
          updated_at
        ) VALUES ($1, $2, $3)
        "#,
    )
    .bind(auth_user_id.to_string())
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await
    .context("failed to insert postgres app user")?;
    sqlx::query(
        r#"
        INSERT INTO user_vehicle_access (
          user_id,
          vehicle_uid,
          access_role,
          created_at,
          updated_at
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(auth_user_id.to_string())
    .bind(&vehicle_uid)
    .bind("owner")
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await
    .context("failed to insert postgres vehicle access")?;

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

    let fetched = crate::handlers::fetch_latest_vehicle_kpis_postgres(
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
    crate::migrations::apply_schema(&sqlite_pool).await?;
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
    let Json(response) = crate::handlers::get_kpis_charging(
        State(state),
        crate::auth::AuthContext::from_user_id(auth_user_id),
        Query(query),
    )
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

#[tokio::test]
async fn postgres_ingest_enforces_idempotency_and_vehicle_ownership_when_env_set() -> Result<()> {
    let Some(ctx) = PostgresTestContext::maybe_new().await? else {
        return Ok(());
    };

    let result = async {
        let state = ctx.app_state().await?;
        let now = Utc::now();
        let owner_user = Uuid::new_v4();
        let foreign_user = Uuid::new_v4();
        let vehicle_uid = Uuid::new_v4();
        let batch_id = Uuid::new_v4();

        let payload = ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
            vec![number_record(
                now - Duration::minutes(1) + Duration::seconds(5),
                "speed.vehicle",
                42.0,
                Some("km/h"),
                None,
            )],
            Vec::new(),
        );

        let Json(first_response) = crate::handlers::post_telemetry_batches(
            State(state.clone()),
            auth_context(owner_user),
            Json(payload),
        )
        .await
        .map_err(|err| anyhow::anyhow!("first postgres ingest failed: {}", err.message))?;
        assert!(first_response.accepted);
        assert!(!first_response.duplicate);
        assert_eq!(first_response.records_accepted, 1);

        let duplicate_payload = ingest_payload(
            vehicle_uid,
            batch_id,
            now - Duration::minutes(2),
            now - Duration::minutes(1),
            vec![number_record(
                now - Duration::minutes(1) + Duration::seconds(5),
                "speed.vehicle",
                42.0,
                Some("km/h"),
                None,
            )],
            Vec::new(),
        );
        let Json(duplicate_response) = crate::handlers::post_telemetry_batches(
            State(state.clone()),
            auth_context(owner_user),
            Json(duplicate_payload),
        )
        .await
        .map_err(|err| anyhow::anyhow!("duplicate postgres ingest failed: {}", err.message))?;
        assert!(duplicate_response.accepted);
        assert!(duplicate_response.duplicate);
        assert_eq!(duplicate_response.records_accepted, 0);

        let foreign_payload = ingest_payload(
            vehicle_uid,
            Uuid::new_v4(),
            now - Duration::minutes(1),
            now,
            vec![number_record(
                now - Duration::seconds(30),
                "speed.vehicle",
                40.0,
                Some("km/h"),
                None,
            )],
            Vec::new(),
        );
        let err = crate::handlers::post_telemetry_batches(
            State(state),
            auth_context(foreign_user),
            Json(foreign_payload),
        )
        .await
        .expect_err("expected ownership conflict for foreign ingest");
        assert_eq!(err.status, StatusCode::FORBIDDEN);
        assert_eq!(err.error, "forbidden");

        let access_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM user_vehicle_access
            WHERE vehicle_uid = $1
            "#,
        )
        .bind(vehicle_uid.to_string())
        .fetch_one(&ctx.pool)
        .await
        .context("failed to count postgres ownership links")?;
        assert_eq!(access_count, 1);

        Ok(())
    }
    .await;

    ctx.cleanup().await?;
    result
}

#[tokio::test]
async fn postgres_rankings_and_temperature_impact_handlers_work_when_env_set() -> Result<()> {
    let Some(ctx) = PostgresTestContext::maybe_new().await? else {
        return Ok(());
    };

    let result = async {
        let state = ctx.app_state().await?;
        let now = Utc::now().to_rfc3339();
        let auth_user_id = Uuid::new_v4();
        let vehicle_uid_owned = Uuid::new_v4().to_string();
        let vehicle_uid_peer = Uuid::new_v4().to_string();

        insert_vehicle_owner_access(
            &ctx.pool,
            &vehicle_uid_owned,
            auth_user_id,
            &now,
            "test_make",
            "test_model",
        )
        .await?;

        sqlx::query(
            r#"
            INSERT INTO vehicle (
              vehicle_uid,
              source_account_id,
              make,
              model,
              powertrain_class,
              created_at,
              updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&vehicle_uid_peer)
        .bind("postgres-test-account")
        .bind("test_make")
        .bind("test_model")
        .bind("bev")
        .bind(&now)
        .bind(&now)
        .execute(&ctx.pool)
        .await
        .context("failed to insert peer vehicle")?;

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
            ) VALUES
              ($1, $2, 'ev_range_efficiency', '30d', 'ev_net_energy_efficiency', 170.0, 'wh_per_km', 'lower_is_better', 'stable', 16, 'all', $3),
              ($4, $5, 'ev_range_efficiency', '30d', 'ev_net_energy_efficiency', 210.0, 'wh_per_km', 'lower_is_better', 'stable', 16, 'all', $3)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_owned)
        .bind(&now)
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_peer)
        .execute(&ctx.pool)
        .await
        .context("failed to insert range KPI snapshots")?;

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
            ) VALUES
              ($1, 'ev_range_efficiency', '30d', 'all', 'make_model:test_make:test_model', 2, 1, $2, 1, 0.88, 'stable', $3),
              ($4, 'ev_range_efficiency', '30d', 'all', 'make_model:test_make:test_model', 2, 1, $5, 2, 0.71, 'stable', $3)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_owned)
        .bind(&now)
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_peer)
        .execute(&ctx.pool)
        .await
        .context("failed to insert ranking snapshots")?;

        let Json(rankings_response) = crate::handlers::get_rankings(
            State(state.clone()),
            auth_context(auth_user_id),
            Query(RankingsQuery {
                ranking_type: "ev_range_efficiency".to_string(),
                timeframe: Some("30d".to_string()),
                temperature_bin: Some("all".to_string()),
                powertrain_class: None,
                make: None,
                model: None,
                trim: None,
                year_band: None,
                region: None,
                limit: Some(10),
                offset: Some(0),
            }),
        )
        .await
        .map_err(|err| anyhow::anyhow!("postgres rankings handler failed: {}", err.message))?;

        assert_eq!(rankings_response.rows.len(), 1);
        assert_eq!(
            rankings_response.rows[0].vehicle_uid,
            Uuid::parse_str(&vehicle_uid_owned)?
        );
        assert!(
            rankings_response.rows[0]
                .kpis
                .contains_key("ev_net_energy_efficiency")
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
              computed_at
            ) VALUES
              ($1, $2, 'ev_temperature_impact', '90d', 'cold_weather_range_retention', 0.86, 'ratio', 'higher_is_better', 'stable', 20, 'cold', 'mild', 'cold', $3),
              ($4, $2, 'ev_temperature_impact', '90d', 'cold_weather_charge_speed_retention', 0.78, 'ratio', 'higher_is_better', 'stable', 20, 'cold', 'mild', 'cold', $3),
              ($5, $6, 'ev_temperature_impact', '90d', 'cold_weather_range_retention', 0.68, 'ratio', 'higher_is_better', 'stable', 20, 'cold', 'mild', 'cold', $3),
              ($7, $6, 'ev_temperature_impact', '90d', 'cold_weather_charge_speed_retention', 0.55, 'ratio', 'higher_is_better', 'stable', 20, 'cold', 'mild', 'cold', $3)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_owned)
        .bind(&now)
        .bind(Uuid::new_v4().to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(&vehicle_uid_peer)
        .bind(Uuid::new_v4().to_string())
        .execute(&ctx.pool)
        .await
        .context("failed to insert temperature KPI snapshots")?;

        let Json(temp_response) = crate::handlers::get_kpis_temperature_impact(
            State(state),
            auth_context(auth_user_id),
            Query(KpiTempQuery {
                vehicle_uid: Uuid::parse_str(&vehicle_uid_owned)?,
                timeframe: Some("90d".to_string()),
                baseline_temperature_bin: Some("mild".to_string()),
                compare_temperature_bin: Some("cold".to_string()),
            }),
        )
        .await
        .map_err(|err| anyhow::anyhow!("postgres temperature-impact handler failed: {}", err.message))?;

        assert!(
            temp_response
                .metrics
                .iter()
                .any(|metric| metric.kpi_key == "cold_weather_range_retention")
        );
        assert!(
            temp_response
                .metrics
                .iter()
                .any(|metric| metric.kpi_key == "cold_weather_charge_speed_retention")
        );
        assert!(temp_response.cohort_benchmark.cohort_size >= 2);

        Ok(())
    }
    .await;

    ctx.cleanup().await?;
    result
}

#[tokio::test]
async fn postgres_internal_job_handler_bridges_inputs_and_outputs_when_env_set() -> Result<()> {
    let Some(ctx) = PostgresTestContext::maybe_new().await? else {
        return Ok(());
    };

    let result = async {
        let state = ctx.app_state().await?;
        let owner_user = Uuid::new_v4();
        let vehicle_uid = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let charge_start = Utc::now() - Duration::minutes(45);
        let charge_stop = Utc::now() - Duration::minutes(15);
        let capture_start = charge_start - Duration::minutes(1);
        let capture_end = charge_stop + Duration::minutes(1);

        let payload = ingest_payload(
            vehicle_uid,
            Uuid::new_v4(),
            capture_start,
            capture_end,
            vec![
                number_record(
                    charge_start + Duration::seconds(5),
                    "ev.soc_pct",
                    20.0,
                    Some("%"),
                    Some(session_id),
                ),
                number_record(
                    charge_stop - Duration::seconds(5),
                    "ev.soc_pct",
                    36.0,
                    Some("%"),
                    Some(session_id),
                ),
                number_record(
                    charge_start + Duration::seconds(10),
                    "ev.charge_power_kw",
                    44.0,
                    Some("kW"),
                    Some(session_id),
                ),
                number_record(
                    charge_stop - Duration::seconds(10),
                    "ev.charge_power_kw",
                    51.0,
                    Some("kW"),
                    Some(session_id),
                ),
                number_record(
                    charge_start + Duration::seconds(15),
                    "environment.ambient_temp_c",
                    16.0,
                    Some("C"),
                    Some(session_id),
                ),
                number_record(
                    charge_stop - Duration::seconds(15),
                    "ev.battery_temp_c",
                    24.0,
                    Some("C"),
                    Some(session_id),
                ),
                string_record(
                    charge_start + Duration::seconds(20),
                    "ev.charger_type",
                    "dc_fast",
                    Some(session_id),
                ),
                string_record(
                    charge_start + Duration::seconds(25),
                    "ev.charging_state",
                    "charging",
                    Some(session_id),
                ),
            ],
            vec![
                SessionEventInput {
                    event_type: "charging_session_start".to_string(),
                    observed_at: charge_start,
                    session_id,
                },
                SessionEventInput {
                    event_type: "charging_session_stop".to_string(),
                    observed_at: charge_stop,
                    session_id,
                },
            ],
        );

        let Json(ingest_response) = crate::handlers::post_telemetry_batches(
            State(state.clone()),
            auth_context(owner_user),
            Json(payload),
        )
        .await
        .map_err(|err| anyhow::anyhow!("postgres job-seed ingest failed: {}", err.message))?;
        assert!(ingest_response.accepted);

        let Json(job_response) = crate::handlers::post_recompute_kpis(State(state.clone()))
            .await
            .map_err(|err| {
                anyhow::anyhow!("postgres internal job handler failed: {}", err.message)
            })?;

        assert!(job_response.ok);
        assert!(job_response.charging_sessions_upserted >= 1);

        let Json(status_response) = crate::handlers::get_latest_job_status(
            State(state),
            Query(JobStatusQuery {
                job_kind: Some("recompute_kpis".to_string()),
            }),
        )
        .await
        .map_err(|err| {
            anyhow::anyhow!("postgres latest-job-status handler failed: {}", err.message)
        })?;
        assert_eq!(status_response.backend, "postgres");
        assert_eq!(status_response.status, "succeeded");
        assert_eq!(
            status_response.response_job_id.as_deref(),
            Some(job_response.job_id.as_str())
        );

        let charging_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM vehicle_charging_session
            WHERE vehicle_uid = $1
            "#,
        )
        .bind(vehicle_uid.to_string())
        .fetch_one(&ctx.pool)
        .await
        .context("failed to count postgres charging-session outputs")?;
        assert!(charging_count >= 1);
        assert_eq!(
            job_response.charging_sessions_upserted as i64,
            charging_count
        );

        let charging_kpi_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM vehicle_kpi_snapshot
            WHERE vehicle_uid = $1
              AND ranking_type = 'ev_charging_performance'
            "#,
        )
        .bind(vehicle_uid.to_string())
        .fetch_one(&ctx.pool)
        .await
        .context("failed to count postgres charging KPI outputs")?;
        assert!(charging_kpi_count >= 2);

        let charging_ranking_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM cohort_ranking_snapshot
            WHERE vehicle_uid = $1
              AND ranking_type = 'ev_charging_performance'
            "#,
        )
        .bind(vehicle_uid.to_string())
        .fetch_one(&ctx.pool)
        .await
        .context("failed to count postgres charging ranking outputs")?;
        assert!(charging_ranking_count >= 1);

        let composite_kpi_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM vehicle_kpi_snapshot
            WHERE vehicle_uid = $1
              AND ranking_type = 'ev_composite'
              AND kpi_key = 'ev_composite_score'
            "#,
        )
        .bind(vehicle_uid.to_string())
        .fetch_one(&ctx.pool)
        .await
        .context("failed to count postgres composite KPI outputs")?;
        assert!(composite_kpi_count >= 1);

        let composite_ranking_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM cohort_ranking_snapshot
            WHERE vehicle_uid = $1
              AND ranking_type = 'ev_composite'
            "#,
        )
        .bind(vehicle_uid.to_string())
        .fetch_one(&ctx.pool)
        .await
        .context("failed to count postgres composite ranking outputs")?;
        assert!(composite_ranking_count >= 1);

        Ok(())
    }
    .await;

    ctx.cleanup().await?;
    result
}

#[tokio::test]
async fn postgres_readiness_handler_returns_family_statuses_when_env_set() -> Result<()> {
    let Some(ctx) = PostgresTestContext::maybe_new().await? else {
        return Ok(());
    };

    let result = async {
        let state = ctx.app_state().await?;
        let now = Utc::now().to_rfc3339();
        let auth_user_id = Uuid::new_v4();
        let vehicle_uid = Uuid::new_v4();

        insert_vehicle_owner_access(
            &ctx.pool,
            &vehicle_uid.to_string(),
            auth_user_id,
            &now,
            "test_make",
            "test_model",
        )
        .await?;

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
        .bind(vehicle_uid.to_string())
        .bind("ev_range_efficiency")
        .bind("90d")
        .bind("ev_net_energy_efficiency")
        .bind(182.0_f64)
        .bind("wh_per_km")
        .bind("lower_is_better")
        .bind("preview")
        .bind(8_i64)
        .bind("all")
        .bind(&now)
        .execute(&ctx.pool)
        .await
        .context("failed to insert postgres readiness range KPI snapshot")?;

        let Json(response) = crate::handlers::get_kpis_readiness(
            State(state),
            auth_context(auth_user_id),
            Query(ReadinessQuery {
                vehicle_uid,
                timeframe: Some("90d".to_string()),
            }),
        )
        .await
        .map_err(|err| anyhow::anyhow!("postgres readiness handler failed: {}", err.message))?;

        assert_eq!(response.families.len(), 4);
        let range_family = response
            .families
            .iter()
            .find(|family| family.ranking_type == "ev_range_efficiency")
            .context("range readiness family missing in postgres response")?;
        assert_eq!(range_family.confidence_level, "preview");
        assert_eq!(range_family.status, "preview");
        assert_eq!(range_family.sample_count, 8);

        Ok(())
    }
    .await;

    ctx.cleanup().await?;
    result
}
