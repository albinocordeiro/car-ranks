use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePoolOptions;

#[test]
fn sqlite_migration_matches_legacy_schema_snapshot() {
    assert_eq!(
        crate::migrations::SQLITE_MIGRATION_0001,
        crate::migrations::LEGACY_SQLITE_SCHEMA
    );
}

#[test]
fn postgres_migration_has_expected_base_tables() {
    assert!(!crate::migrations::POSTGRES_MIGRATION_0001.contains("PRAGMA"));
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
            crate::migrations::POSTGRES_MIGRATION_0001.contains(&marker),
            "missing table in postgres migration: {}",
            table_name
        );
    }
}

#[test]
fn ownership_migrations_define_expected_tables() {
    for migration in [
        crate::migrations::SQLITE_MIGRATION_0002,
        crate::migrations::POSTGRES_MIGRATION_0002,
    ] {
        assert!(migration.contains("CREATE TABLE IF NOT EXISTS app_user"));
        assert!(migration.contains("CREATE TABLE IF NOT EXISTS user_vehicle_access"));
    }
}

#[test]
fn internal_job_run_migrations_define_expected_table() {
    for migration in [
        crate::migrations::SQLITE_MIGRATION_0003,
        crate::migrations::POSTGRES_MIGRATION_0003,
    ] {
        assert!(migration.contains("CREATE TABLE IF NOT EXISTS internal_job_run"));
    }
}

#[test]
fn internal_job_lock_migrations_define_expected_table() {
    for migration in [
        crate::migrations::SQLITE_MIGRATION_0004,
        crate::migrations::POSTGRES_MIGRATION_0004,
    ] {
        assert!(migration.contains("CREATE TABLE IF NOT EXISTS internal_job_lock"));
    }
}

#[tokio::test]
async fn apply_schema_records_migrations_once() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect in-memory sqlite")?;

    crate::migrations::apply_schema(&pool).await?;
    crate::migrations::apply_schema(&pool).await?;

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

    let ownership_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM schema_migration
        WHERE migration_id = '0002_auth_ownership'
          AND backend = 'sqlite'
        "#,
    )
    .fetch_one(&pool)
    .await
    .context("failed to count ownership migration")?;

    assert_eq!(ownership_count, 1);

    let job_run_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM schema_migration
        WHERE migration_id = '0003_internal_job_runs'
          AND backend = 'sqlite'
        "#,
    )
    .fetch_one(&pool)
    .await
    .context("failed to count internal job run migration")?;

    assert_eq!(job_run_count, 1);

    let job_lock_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM schema_migration
        WHERE migration_id = '0004_internal_job_locks'
          AND backend = 'sqlite'
        "#,
    )
    .fetch_one(&pool)
    .await
    .context("failed to count internal job lock migration")?;

    assert_eq!(job_lock_count, 1);
    Ok(())
}
