use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Query, State};
use chrono::{Duration, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Connection, Executor};
use uuid::Uuid;

use super::*;

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
    let Json(response) = crate::handlers::get_kpis_charging(State(state), Query(query))
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
